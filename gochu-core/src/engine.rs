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

        // Delete exactly one *displayed* character from the right, even if it
        // was produced by multiple Telex keystrokes (tone + vowel mods).
        //
        // We reconstruct, for each raw key, which buffer indices it touched,
        // then drop all keys that affected the last character. This keeps
        // tone on the preceding vowel in cases like \"phúc\" (typed \"phucs\"):
        // Backspace removes the trailing \"c\" but preserves \"phú\".
        let last_idx = self.buf.len().saturating_sub(1);

        let mut sim_buf: Vec<char> = Vec::new();
        let mut touched: Vec<Vec<usize>> = Vec::with_capacity(self.raw.len());

        for &key in &self.raw {
            let effect = transform::classify_key(key, &sim_buf);
            let mut indices: alloc::vec::Vec<usize> = alloc::vec::Vec::new();

            match effect {
                KeyEffect::ToneApplied { position, .. }
                | KeyEffect::DdApplied { position, .. }
                | KeyEffect::VowelModified { position, .. } => {
                    if position < sim_buf.len() {
                        indices.push(position);
                    }
                }
                KeyEffect::WAsVowel(_) | KeyEffect::Append(_) => {
                    indices.push(sim_buf.len());
                }
                KeyEffect::ToneClearAndAppend { position, .. } => {
                    if position < sim_buf.len() {
                        indices.push(position);
                    }
                    indices.push(sim_buf.len());
                }
                KeyEffect::Commit(_) | KeyEffect::Backspace => {}
            }

            sim_buf = transform::apply_effect(&sim_buf, &effect);
            touched.push(indices);
        }

        // Filter out all raw keystrokes that touched the last character.
        let mut new_raw = Vec::with_capacity(self.raw.len());
        for (i, ch) in self.raw.iter().enumerate() {
            if !touched
                .get(i)
                .map(|idxs| idxs.contains(&last_idx))
                .unwrap_or(false)
            {
                new_raw.push(*ch);
            }
        }
        // If filtering somehow removed nothing (should not happen), fall back
        // to the previous behaviour of popping raw keys until the length
        // shrinks by one, as a safety net.
        if new_raw.len() == self.raw.len() {
            let target_len = self.buf.len().saturating_sub(1);
            while !self.raw.is_empty() {
                self.raw.pop();
                self.buf = transform::replay(&self.raw);
                if self.buf.len() <= target_len {
                    break;
                }
            }
        } else {
            self.raw = new_raw;
            self.buf = transform::replay(&self.raw);
        }

        if self.buf.is_empty() {
            self.composing = false;
            Action::Composing(String::new())
        } else {
            self.composing = true;
            Action::Composing(self.get_display())
        }
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
        e.reset();
        assert_eq!(type_word(&mut e, "Dd"), "Đ");
        e.reset();
        assert_eq!(type_word(&mut e, "dD"), "đ");
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
    fn backspace_deletes_one_composed_char() {
        let mut e = TelexEngine::new();
        type_word(&mut e, "as"); // á
        let result = type_word(&mut e, "\x08"); // delete the á
        assert_eq!(result, "");
    }

    #[test]
    fn backspace_on_circumflex_tone_deletes_entire_char() {
        let mut e = TelexEngine::new();
        // \"côj\" → \"cộ\" (ô + nặng)
        assert_eq!(type_word(&mut e, "coj"), "cọ");
        e.reset();
        assert_eq!(type_word(&mut e, "cooxj"), "cộ");
        let result = type_word(&mut e, "\x08");
        assert_eq!(result, "c");
    }

    #[test]
    fn backspace_after_final_consonant_keeps_tone() {
        let mut e = TelexEngine::new();
        assert_eq!(type_word(&mut e, "phucs"), "phúc");
        let result = type_word(&mut e, "\x08");
        assert_eq!(result, "phú");
    }

    #[test]
    fn backspace_through_vowel_mod() {
        let mut e = TelexEngine::new();
        type_word(&mut e, "aa"); // â
        let result = type_word(&mut e, "\x08"); // delete â
        assert_eq!(result, "");
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
        let result = type_word(&mut e, "\x08");
        assert_eq!(result, "Việ"); // delete 't'
        let result = type_word(&mut e, "\x08");
        assert_eq!(result, "Vi"); // delete the toned vowel
        let result = type_word(&mut e, "\x08");
        assert_eq!(result, "V"); // delete the i
    }

    #[test]
    fn six_and_sixx_behavior() {
        let mut e = TelexEngine::new();
        // \"six\" → sĩ
        assert_eq!(type_word(&mut e, "six"), "sĩ");
        e.reset();
        // \"sixx\" → six (double x cancels the tone but keeps one x)
        assert_eq!(type_word(&mut e, "sixx"), "six");
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
