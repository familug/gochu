use gochu_core::{Action, TelexEngine};
use zbus::zvariant::{DynamicType, OwnedObjectPath, Value};
use zbus::{interface, Connection, Message};

use crate::text::ibus_text;

const IBUS_RELEASE_MASK: u32 = 1 << 30;
const IBUS_CONTROL_MASK: u32 = 1 << 2;
const IBUS_MOD1_MASK: u32 = 1 << 3;
const IBUS_SUPER_MASK: u32 = 1 << 26;

const XK_BACKSPACE: u32 = 0xff08;
const XK_RETURN: u32 = 0xff0d;
const XK_ESCAPE: u32 = 0xff1b;
const XK_KP_ENTER: u32 = 0xff8d;

const ENGINE_IFACE: &str = "org.freedesktop.IBus.Engine";

pub struct GochuEngine {
    telex: TelexEngine,
    conn: Connection,
    path: String,
}

impl GochuEngine {
    pub fn new(conn: Connection, path: String) -> Self {
        Self {
            telex: TelexEngine::new(),
            conn,
            path,
        }
    }

    async fn send_signal<B: serde::Serialize + DynamicType>(
        &self,
        signal: &str,
        body: &B,
    ) {
        let sig = body.dynamic_signature();
        match Message::signal(
            self.path.as_str(),
            ENGINE_IFACE,
            signal,
        ) {
            Ok(builder) => match builder.build(body) {
                Ok(msg) => {
                    if let Err(e) = self.conn.send(&msg).await {
                        crate::log(&format!("{signal} send error: {e}"));
                    }
                }
                Err(e) => crate::log(&format!("{signal} build error (sig={sig}): {e}")),
            },
            Err(e) => crate::log(&format!("{signal} builder error: {e}")),
        }
    }

    async fn commit(&self, text: &str) {
        if !text.is_empty() {
            self.send_signal("CommitText", &ibus_text(text)).await;
        }
    }

    async fn preedit(&self, text: &str) {
        if text.is_empty() {
            self.send_signal("HidePreeditText", &()).await;
        } else {
            let cursor = text.chars().count() as u32;
            let mode = 0u32; // IBUS_ENGINE_PREEDIT_CLEAR
            self.send_signal(
                "UpdatePreeditText",
                &(ibus_text(text), cursor, true, mode),
            )
            .await;
        }
    }

    async fn hide_preedit(&self) {
        self.send_signal("HidePreeditText", &()).await;
    }

    async fn flush(&mut self) {
        if !self.telex.is_composing() {
            return;
        }
        let pending = self.telex.get_display();
        self.telex.reset();
        self.hide_preedit().await;
        if !pending.is_empty() {
            self.commit(&pending).await;
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl GochuEngine {
    async fn process_key_event(
        &mut self,
        keyval: u32,
        _keycode: u32,
        state: u32,
    ) -> bool {
        if state & IBUS_RELEASE_MASK != 0 {
            return false;
        }

        if state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_SUPER_MASK) != 0 {
            self.flush().await;
            return false;
        }

        match keyval {
            XK_RETURN | XK_KP_ENTER => {
                self.flush().await;
                return false;
            }
            XK_ESCAPE => {
                self.telex.reset();
                self.hide_preedit().await;
                return false;
            }
            _ => {}
        }

        let ch = match keyval {
            XK_BACKSPACE => '\x08',
            0x20..=0x7e => keyval as u8 as char,
            _ => {
                self.flush().await;
                return false;
            }
        };

        match self.telex.process_key(ch) {
            Action::Composing(text) => {
                self.preedit(&text).await;
                true
            }
            Action::Commit(text) => {
                // Contract with gochu-core: Commit(\"\") on backspace when the
                // composing buffer is empty means \"let the client handle
                // deletion\". In that case we must return false so IBus
                // forwards Backspace to the application.
                if text.is_empty() {
                    self.hide_preedit().await;
                    false
                } else {
                    self.hide_preedit().await;
                    self.commit(&text).await;
                    true
                }
            }
        }
    }

    fn focus_in(&mut self) {}

    async fn focus_out(&mut self) {
        self.flush().await;
    }

    async fn reset(&mut self) {
        self.flush().await;
    }

    fn enable(&mut self) {}

    async fn disable(&mut self) {
        self.flush().await;
    }

    fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    fn set_capabilities(&self, _cap: u32) {}
    fn page_up(&self) -> bool { false }
    fn page_down(&self) -> bool { false }
    fn cursor_up(&self) -> bool { false }
    fn cursor_down(&self) -> bool { false }
    fn property_activate(&self, _name: &str, _state: u32) {}
    fn set_content_type(&self, _purpose: u32, _hints: u32) {}
    fn set_surrounding_text(&self, _text: Value<'_>, _cursor_index: u32, _anchor_pos: u32) {}
    fn destroy(&mut self) {}
}

// -- Factory: IBus calls CreateEngine to instantiate engines --

pub struct GochuFactory {
    conn: Connection,
    engine_count: u32,
}

impl GochuFactory {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
            engine_count: 0,
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl GochuFactory {
    async fn create_engine(
        &mut self,
        engine_name: &str,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let n = self.engine_count;
        self.engine_count += 1;

        let path = format!("/org/freedesktop/IBus/Engine/{n}");
        crate::log(&format!("CreateEngine({engine_name:?}) -> {path}"));

        let engine = GochuEngine::new(self.conn.clone(), path.clone());

        self.conn
            .object_server()
            .at(&*path, engine)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        OwnedObjectPath::try_from(path)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    fn destroy(&self) {}
}
