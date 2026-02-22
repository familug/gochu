use gochu_core::{Action, TelexEngine};
use zbus::object_server::SignalContext;
use zbus::zvariant::{OwnedObjectPath, Value};
use zbus::{interface, Connection};

use crate::text::ibus_text;

const IBUS_RELEASE_MASK: u32 = 1 << 30;
const IBUS_CONTROL_MASK: u32 = 1 << 2;
const IBUS_MOD1_MASK: u32 = 1 << 3;
const IBUS_SUPER_MASK: u32 = 1 << 26;

const XK_BACKSPACE: u32 = 0xff08;
const XK_RETURN: u32 = 0xff0d;
const XK_ESCAPE: u32 = 0xff1b;
const XK_KP_ENTER: u32 = 0xff8d;

pub struct GochuEngine {
    telex: TelexEngine,
}

impl GochuEngine {
    pub fn new() -> Self {
        Self {
            telex: TelexEngine::new(),
        }
    }

    async fn commit(&self, ctxt: &SignalContext<'_>, text: &str) {
        if !text.is_empty() {
            let _ = Self::commit_text(ctxt, ibus_text(text)).await;
        }
    }

    async fn preedit(&self, ctxt: &SignalContext<'_>, text: &str) {
        if text.is_empty() {
            let _ = Self::hide_preedit_text(ctxt).await;
        } else {
            let cursor = text.chars().count() as u32;
            let _ =
                Self::update_preedit_text(ctxt, ibus_text(text), cursor, true).await;
        }
    }

    /// Commit any pending composing text and reset the engine.
    async fn flush(&mut self, ctxt: &SignalContext<'_>) {
        if !self.telex.is_composing() {
            return;
        }
        let pending = self.telex.get_display();
        self.telex.reset();
        let _ = Self::hide_preedit_text(ctxt).await;
        if !pending.is_empty() {
            self.commit(ctxt, &pending).await;
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl GochuEngine {
    async fn process_key_event(
        &mut self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
        keyval: u32,
        _keycode: u32,
        state: u32,
    ) -> bool {
        if state & IBUS_RELEASE_MASK != 0 {
            return false;
        }

        // Modifier combos: flush composing text, let IBus handle the key
        if state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_SUPER_MASK) != 0 {
            self.flush(&ctxt).await;
            return false;
        }

        match keyval {
            XK_RETURN | XK_KP_ENTER => {
                self.flush(&ctxt).await;
                return false;
            }
            XK_ESCAPE => {
                self.telex.reset();
                let _ = Self::hide_preedit_text(&ctxt).await;
                return false;
            }
            _ => {}
        }

        let ch = match keyval {
            XK_BACKSPACE => '\x08',
            0x20..=0x7e => keyval as u8 as char,
            _ => {
                // Unknown key (arrows, F-keys, etc.): flush and forward
                self.flush(&ctxt).await;
                return false;
            }
        };

        match self.telex.process_key(ch) {
            Action::Composing(text) => {
                self.preedit(&ctxt, &text).await;
                true
            }
            Action::Commit(text) => {
                let _ = Self::hide_preedit_text(&ctxt).await;
                self.commit(&ctxt, &text).await;
                true
            }
        }
    }

    fn focus_in(&mut self) {}

    async fn focus_out(
        &mut self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) {
        self.flush(&ctxt).await;
    }

    async fn reset(
        &mut self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) {
        self.flush(&ctxt).await;
    }

    fn enable(&mut self) {}

    async fn disable(
        &mut self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) {
        self.flush(&ctxt).await;
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

    #[zbus(signal)]
    async fn commit_text(
        ctxt: &SignalContext<'_>,
        text: Value<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_preedit_text(
        ctxt: &SignalContext<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
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
        _engine_name: &str,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let n = self.engine_count;
        self.engine_count += 1;

        let path = format!("/org/freedesktop/IBus/Engine/{n}");
        let engine = GochuEngine::new();

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
