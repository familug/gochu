use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_node_experimental);

use gochu_wasm::Gochu;

fn get_str(obj: &JsValue, key: &str) -> String {
    js_sys::Reflect::get(obj, &key.into())
        .unwrap()
        .as_string()
        .unwrap()
}

#[wasm_bindgen_test]
fn new_engine_not_composing() {
    let g = Gochu::new();
    assert!(!g.is_composing());
    assert_eq!(g.get_display(), "");
}

#[wasm_bindgen_test]
fn process_key_returns_composing() {
    let mut g = Gochu::new();
    let result = g.process_key('a');
    assert_eq!(get_str(&result, "type"), "composing");
    assert_eq!(get_str(&result, "text"), "a");
    assert!(g.is_composing());
}

#[wasm_bindgen_test]
fn process_key_tone_applied() {
    let mut g = Gochu::new();
    g.process_key('a');
    let result = g.process_key('s');
    assert_eq!(get_str(&result, "type"), "composing");
    assert_eq!(get_str(&result, "text"), "á");
}

#[wasm_bindgen_test]
fn space_commits() {
    let mut g = Gochu::new();
    g.process_key('a');
    g.process_key('s');
    let result = g.process_key(' ');
    assert_eq!(get_str(&result, "type"), "commit");
    assert_eq!(get_str(&result, "text"), "á ");
    assert!(!g.is_composing());
}

#[wasm_bindgen_test]
fn reset_clears_state() {
    let mut g = Gochu::new();
    g.process_key('V');
    g.process_key('i');
    assert!(g.is_composing());

    g.reset();
    assert!(!g.is_composing());
    assert_eq!(g.get_display(), "");
}

#[wasm_bindgen_test]
fn full_word_viet() {
    let mut g = Gochu::new();
    for c in "Vieejt".chars() {
        g.process_key(c);
    }
    assert_eq!(g.get_display(), "Việt");
}

#[wasm_bindgen_test]
fn dd_produces_d_stroke() {
    let mut g = Gochu::new();
    g.process_key('d');
    g.process_key('d');
    assert_eq!(g.get_display(), "đ");
}
