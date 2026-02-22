//! Pure transformation functions. No mutable state — every function takes
//! input and returns output, making them independently testable.

extern crate alloc;
use alloc::vec::Vec;

use crate::tone::{apply_tone, Tone};
use crate::vowel::{is_vowel, modify_vowel};

/// What effect a single keystroke has on a composing buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEffect {
    /// A tone was applied at `buf[position]`
    ToneApplied { position: usize, replacement: char },
    /// dd → đ: the last char was replaced
    DdApplied { position: usize, replacement: char },
    /// A vowel was modified in-place (e.g. a→â)
    VowelModified { position: usize, replacement: char },
    /// Standalone w → ư (or W → Ư) appended
    WAsVowel(char),
    /// Regular character appended
    Append(char),
    /// Key triggers a commit (space, punctuation, non-alpha)
    Commit(char),
    /// Backspace
    Backspace,
}

/// Pure: classify a key given the current buffer contents.
/// Does NOT mutate anything.
pub fn classify_key(key: char, buf: &[char]) -> KeyEffect {
    if key == '\x08' || key == '\x7f' {
        return KeyEffect::Backspace;
    }

    if is_word_separator(key) || !key.is_ascii_alphabetic() {
        return KeyEffect::Commit(key);
    }

    if let Some(effect) = try_tone(key, buf) {
        return effect;
    }

    if let Some(effect) = try_dd(key, buf) {
        return effect;
    }

    if let Some(effect) = try_vowel_modify(key, buf) {
        return effect;
    }

    if key == 'w' || key == 'W' {
        return try_w(key, buf);
    }

    KeyEffect::Append(key)
}

fn try_tone(key: char, buf: &[char]) -> Option<KeyEffect> {
    let tone = Tone::from_telex(key)?;
    let has_vowel = buf.iter().any(|c| is_vowel(*c));
    if !has_vowel {
        return None;
    }
    let pos = crate::vowel::tone_position(buf)?;
    let replacement = apply_tone(buf[pos], tone);
    Some(KeyEffect::ToneApplied {
        position: pos,
        replacement,
    })
}

fn try_dd(key: char, buf: &[char]) -> Option<KeyEffect> {
    let last = *buf.last()?;
    match (last, key) {
        ('d', 'd') => Some(KeyEffect::DdApplied {
            position: buf.len() - 1,
            replacement: 'đ',
        }),
        ('D', 'D') => Some(KeyEffect::DdApplied {
            position: buf.len() - 1,
            replacement: 'Đ',
        }),
        _ => None,
    }
}

fn try_vowel_modify(key: char, buf: &[char]) -> Option<KeyEffect> {
    let lower = key.to_ascii_lowercase();
    if !matches!(lower, 'a' | 'e' | 'o' | 'w') {
        return None;
    }

    for i in (0..buf.len()).rev() {
        let c = buf[i];
        if let Some(modified) = modify_vowel(c, key) {
            return Some(KeyEffect::VowelModified {
                position: i,
                replacement: modified,
            });
        }
        if !is_vowel(c) {
            break;
        }
    }
    None
}

fn try_w(key: char, buf: &[char]) -> KeyEffect {
    // First try modifying existing u/o
    for i in (0..buf.len()).rev() {
        let c = buf[i];
        if let Some(modified) = modify_vowel(c, key) {
            return KeyEffect::VowelModified {
                position: i,
                replacement: modified,
            };
        }
        if !is_vowel(c) {
            break;
        }
    }
    let ch = if key == 'W' { 'Ư' } else { 'ư' };
    KeyEffect::WAsVowel(ch)
}

/// Pure: apply an effect to a buffer, returning a new buffer.
pub fn apply_effect(buf: &[char], effect: &KeyEffect) -> Vec<char> {
    let mut result: Vec<char> = buf.to_vec();
    match *effect {
        KeyEffect::ToneApplied {
            position,
            replacement,
        }
        | KeyEffect::DdApplied {
            position,
            replacement,
        }
        | KeyEffect::VowelModified {
            position,
            replacement,
        } => {
            if position < result.len() {
                result[position] = replacement;
            }
        }
        KeyEffect::WAsVowel(ch) | KeyEffect::Append(ch) => {
            result.push(ch);
        }
        KeyEffect::Commit(_) | KeyEffect::Backspace => {}
    }
    result
}

/// Pure: replay a sequence of raw keys, returning the resulting buffer.
pub fn replay(raw_keys: &[char]) -> Vec<char> {
    let mut buf = Vec::new();
    for &key in raw_keys {
        let effect = classify_key(key, &buf);
        buf = apply_effect(&buf, &effect);
    }
    buf
}

fn is_word_separator(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\t' | '\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_tone_on_vowel() {
        let buf: Vec<char> = vec!['a'];
        let effect = classify_key('s', &buf);
        assert_eq!(
            effect,
            KeyEffect::ToneApplied {
                position: 0,
                replacement: 'á'
            }
        );
    }

    #[test]
    fn classify_tone_no_vowel_falls_through() {
        let buf: Vec<char> = vec!['d'];
        let effect = classify_key('s', &buf);
        assert_eq!(effect, KeyEffect::Append('s'));
    }

    #[test]
    fn classify_dd() {
        let buf: Vec<char> = vec!['d'];
        assert_eq!(
            classify_key('d', &buf),
            KeyEffect::DdApplied {
                position: 0,
                replacement: 'đ'
            }
        );
    }

    #[test]
    fn classify_vowel_modify() {
        let buf: Vec<char> = vec!['a'];
        assert_eq!(
            classify_key('a', &buf),
            KeyEffect::VowelModified {
                position: 0,
                replacement: 'â'
            }
        );
    }

    #[test]
    fn classify_w_modifies_u() {
        let buf: Vec<char> = vec!['u'];
        assert_eq!(
            classify_key('w', &buf),
            KeyEffect::VowelModified {
                position: 0,
                replacement: 'ư'
            }
        );
    }

    #[test]
    fn classify_w_standalone() {
        let buf: Vec<char> = vec!['t'];
        assert_eq!(classify_key('w', &buf), KeyEffect::WAsVowel('ư'));
    }

    #[test]
    fn classify_commit_on_space() {
        let buf: Vec<char> = vec!['a'];
        assert_eq!(classify_key(' ', &buf), KeyEffect::Commit(' '));
    }

    #[test]
    fn classify_commit_on_digit() {
        let buf: Vec<char> = vec![];
        assert_eq!(classify_key('1', &buf), KeyEffect::Commit('1'));
    }

    #[test]
    fn apply_effect_tone() {
        let buf = vec!['t', 'o', 'i'];
        let effect = KeyEffect::ToneApplied {
            position: 1,
            replacement: 'ó',
        };
        assert_eq!(apply_effect(&buf, &effect), vec!['t', 'ó', 'i']);
    }

    #[test]
    fn apply_effect_append() {
        let buf = vec!['x'];
        assert_eq!(apply_effect(&buf, &KeyEffect::Append('i')), vec!['x', 'i']);
    }

    #[test]
    fn replay_viet() {
        let raw: Vec<char> = "Vieejt".chars().collect();
        let result: String = replay(&raw).into_iter().collect();
        assert_eq!(result, "Việt");
    }

    #[test]
    fn replay_dde() {
        let raw: Vec<char> = "ddeef".chars().collect();
        let result: String = replay(&raw).into_iter().collect();
        assert_eq!(result, "đề");
    }

    #[test]
    fn replay_empty() {
        assert_eq!(replay(&[]), Vec::<char>::new());
    }
}
