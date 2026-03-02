use gochu_core::{Action, TelexEngine};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Gochu {
    engine: TelexEngine,
}

impl Default for Gochu {
    fn default() -> Self {
        Self {
            engine: TelexEngine::new(),
        }
    }
}

#[wasm_bindgen]
impl Gochu {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a single character. Returns a JS object: { type: "composing"|"commit", text: "..." }
    pub fn process_key(&mut self, key: char) -> JsValue {
        let action = self.engine.process_key(key);
        let (action_type, text) = match action {
            Action::Composing(s) => ("composing", s),
            Action::Commit(s) => ("commit", s),
        };
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"type".into(), &action_type.into()).unwrap();
        js_sys::Reflect::set(&obj, &"text".into(), &text.into()).unwrap();
        obj.into()
    }

    pub fn get_display(&self) -> String {
        self.engine.get_display()
    }

    pub fn is_composing(&self) -> bool {
        self.engine.is_composing()
    }

    pub fn reset(&mut self) {
        self.engine.reset();
    }
}
