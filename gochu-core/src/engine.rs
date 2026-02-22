//! Imperative shell: manages mutable state, delegates all decisions
//! to the pure functions in `transform`.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::transform::{self, KeyEffect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Composing(String),
    Commit(String),
}

/// Thin stateful wrapper around the pure Telex transformation functions.
/// All business logic lives in `transform`; this struct only manages
/// the mutable buffer and raw-key history.
#[derive(Debug, Clone)]
pub struct TelexEngine {
    buf: Vec<char>,
    raw: Vec<char>,
    composing: bool,
}

impl TelexEngine {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            raw: Vec::new(),
            composing: false,
        }
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.raw.clear();
        self.composing = false;
    }

    pub fn get_display(&self) -> String {
        self.buf.iter().collect()
    }

    pub fn get_raw(&self) -> String {
        self.raw.iter().collect()
    }

    pub fn is_composing(&self) -> bool {
        self.composing
    }

    pub fn process_key(&mut self, key: char) -> Action {
        let effect = transform::classify_key(key, &self.buf);

        match effect {
            KeyEffect::Backspace => self.handle_backspace(),
            KeyEffect::Commit(ch) => {
                let mut result = self.get_display();
                result.push(ch);
                self.reset();
                Action::Commit(result)
            }
            _ => {
                self.composing = true;
                self.raw.push(key);
                self.buf = transform::apply_effect(&self.buf, &effect);
                Action::Composing(self.get_display())
            }
        }
    }

    fn handle_backspace(&mut self) -> Action {
        if self.buf.is_empty() {
            return Action::Commit(String::new());
        }

        self.raw.pop();

        if self.raw.is_empty() {
            self.buf.clear();
            self.composing = false;
            return Action::Composing(String::new());
        }

        self.buf = transform::replay(&self.raw);
        self.composing = true;
        Action::Composing(self.get_display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_word(engine: &mut TelexEngine, word: &str) -> String {
        let mut result = String::new();
        for c in word.chars() {
            match engine.process_key(c) {
                Action::Composing(s) => result = s,
                Action::Commit(s) => result = s,
            }
        }
        result
    }

    // -- vowel modification --

    #[test]
    fn basic_vowels() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "aa"), "â");
        e.reset();
        assert_eq!(type_word(&mut e, "ee"), "ê");
        e.reset();
        assert_eq!(type_word(&mut e, "oo"), "ô");
        e.reset();
        assert_eq!(type_word(&mut e, "ow"), "ơ");
        e.reset();
        assert_eq!(type_word(&mut e, "uw"), "ư");
        e.reset();
        assert_eq!(type_word(&mut e, "aw"), "ă");
    }

    #[test]
    fn uppercase_vowels() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "AA"), "Â");
        e.reset();
        assert_eq!(type_word(&mut e, "EE"), "Ê");
        e.reset();
        assert_eq!(type_word(&mut e, "OO"), "Ô");
        e.reset();
        assert_eq!(type_word(&mut e, "OW"), "Ơ");
        e.reset();
        assert_eq!(type_word(&mut e, "UW"), "Ư");
    }

    // -- dd --

    #[test]
    fn dd_lowercase_uppercase() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "dd"), "đ");
        e.reset();
        assert_eq!(type_word(&mut e, "DD"), "Đ");
    }

    // -- tones --

    #[test]
    fn all_tones_on_a() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "as"), "á");
        e.reset();
        assert_eq!(type_word(&mut e, "af"), "à");
        e.reset();
        assert_eq!(type_word(&mut e, "ar"), "ả");
        e.reset();
        assert_eq!(type_word(&mut e, "ax"), "ã");
        e.reset();
        assert_eq!(type_word(&mut e, "aj"), "ạ");
    }

    #[test]
    fn tone_removal_with_z() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "asz"), "a");
    }

    #[test]
    fn tone_replacement() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "asf"), "à");
    }

    // -- combined words --

    #[test]
    fn combined_viet() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "Vieejt"), "Việt");
    }

    #[test]
    fn combined_dde() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "ddeef"), "đề");
    }

    #[test]
    fn combined_uong() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "uwowng"), "ương");
    }

    #[test]
    fn combined_nguoi() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "nguwowif"), "người");
    }

    #[test]
    fn combined_tuong() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "tuwowng"), "tương");
    }

    // -- commit behavior --

    #[test]
    fn space_commits() {
        let mut e = TelexEngine::new();
        let result = type_word(&mut e, "xin ");
        assert_eq!(result, "xin ");
        assert!(!e.is_composing());
    }

    #[test]
    fn punctuation_commits() {
        let mut e = TelexEngine::new();
        let result = type_word(&mut e, "xin.");
        assert_eq!(result, "xin.");
        assert!(!e.is_composing());
    }

    #[test]
    fn digit_commits() {
        let mut e = TelexEngine::new();
        let result = type_word(&mut e, "abc1");
        assert_eq!(result, "abc1");
        assert!(!e.is_composing());
    }

    // -- backspace --

    #[test]
    fn backspace_removes_last_raw_key() {
        let mut e = TelexEngine::new();
        type_word(&mut e, "as"); // á
        let result = type_word(&mut e, "\x08"); // remove the 's'
        assert_eq!(result, "a");
    }

    #[test]
    fn backspace_through_vowel_mod() {
        let mut e = TelexEngine::new();
        type_word(&mut e, "aa"); // â
        let result = type_word(&mut e, "\x08"); // remove one 'a'
        assert_eq!(result, "a");
    }

    #[test]
    fn backspace_to_empty() {
        let mut e = TelexEngine::new();
        type_word(&mut e, "a");
        let result = type_word(&mut e, "\x08");
        assert_eq!(result, "");
        assert!(!e.is_composing());
    }

    #[test]
    fn backspace_on_empty_commits() {
        let mut e = TelexEngine::new();
        match e.process_key('\x08') {
            Action::Commit(s) => assert_eq!(s, ""),
            other => panic!("expected Commit, got {:?}", other),
        }
    }

    #[test]
    fn multiple_backspaces() {
        let mut e = TelexEngine::new();
        type_word(&mut e, "Vieejt"); // Việt (raw: V,i,e,e,j,t)
        type_word(&mut e, "\x08"); // raw: V,i,e,e,j → Việ
        let result = type_word(&mut e, "\x08"); // raw: V,i,e,e → Viê
        assert_eq!(result, "Viê");
        let result = type_word(&mut e, "\x08"); // raw: V,i,e → Vie
        assert_eq!(result, "Vie");
    }

    // -- passthrough: tone key without vowel --

    #[test]
    fn tone_key_without_vowel_is_literal() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "s"), "s");
    }

    #[test]
    fn d_alone_is_literal() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "d"), "d");
    }

    // -- standalone w --

    #[test]
    fn standalone_w() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "w"), "ư");
        e.reset();
        assert_eq!(type_word(&mut e, "W"), "Ư");
    }

    // -- multi-word sequences --

    #[test]
    fn multi_word() {
        let mut e = TelexEngine::new();
        type_word(&mut e, "xin ");
        assert!(!e.is_composing());
        let result = type_word(&mut e, "chaof");
        assert_eq!(result, "chào");
    }

    // -- tone placement in clusters --

    #[test]
    fn tone_on_modified_vowel_in_cluster() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "thaanf"), "thần");
    }

    #[test]
    fn tone_with_final_consonant() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "toans"), "toán");
    }

    #[test]
    fn tone_without_final_consonant() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "toas"), "tóa");
    }
}
