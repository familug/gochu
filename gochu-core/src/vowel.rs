extern crate alloc;
use alloc::vec::Vec;

use crate::tone::{apply_tone, get_tone, strip_tone, Tone};

pub fn is_vowel(c: char) -> bool {
    let base = strip_tone(c);
    let lower = if base.is_ascii() {
        base.to_ascii_lowercase()
    } else {
        base
    };
    matches!(lower, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
        || is_modified_vowel(c)
}

pub fn is_modified_vowel(c: char) -> bool {
    let base = strip_tone(c);
    matches!(
        base,
        'ă' | 'â' | 'ê' | 'ô' | 'ơ' | 'ư' | 'Ă' | 'Â' | 'Ê' | 'Ô' | 'Ơ' | 'Ư'
    )
}

/// Try to apply a vowel modifier (w, a, e, o) to a character.
/// Returns Some(modified) if applicable.
pub fn modify_vowel(base: char, modifier: char) -> Option<char> {
    let tone = get_tone(base);
    let stripped = strip_tone(base);
    let lower_stripped = stripped.to_ascii_lowercase();
    let lower_mod = modifier.to_ascii_lowercase();

    let new_base = match (lower_stripped, lower_mod) {
        ('a', 'a') => 'â',
        ('a', 'w') => 'ă',
        ('e', 'e') => 'ê',
        ('o', 'o') => 'ô',
        ('o', 'w') => 'ơ',
        ('u', 'w') => 'ư',
        _ => return None,
    };

    let new_base = if stripped.is_uppercase() {
        new_base.to_uppercase().next().unwrap()
    } else {
        new_base
    };

    Some(apply_tone(new_base, tone))
}

/// Find the index of the vowel that should receive the tone mark.
/// `vowel_indices` is a list of indices into the buffer that are vowels.
/// Returns the index into the buffer.
pub fn tone_position(buf: &[char]) -> Option<usize> {
    let mut vowel_positions: Vec<usize> = buf
        .iter()
        .enumerate()
        .filter(|(_, c)| is_vowel(**c))
        .map(|(i, _)| i)
        .collect();

    if vowel_positions.is_empty() {
        return None;
    }

    // Treat leading "gi" / "qu" as consonant clusters when followed by
    // another vowel, so tones fall on the main vowel:
    // - "gias" → "giá" (tone on 'a', not 'i')
    // - "quas" → "quá" (tone on 'a', not 'u')
    if vowel_positions.len() >= 2 {
        if let Some(first) = vowel_positions.first().copied() {
            if first == 1 {
                let first_char = buf.first().copied().unwrap_or_default();
                let second_char = buf.get(1).copied().unwrap_or_default();
                let is_gi_cluster = matches!(first_char, 'g' | 'G')
                    && matches!(second_char, 'i' | 'I');
                let is_qu_cluster = matches!(first_char, 'q' | 'Q')
                    && matches!(second_char, 'u' | 'U');

                if is_gi_cluster || is_qu_cluster {
                    // Drop the glide vowel from consideration.
                    vowel_positions.remove(0);
                }
            }
        }
    }

    if vowel_positions.len() == 1 {
        return Some(vowel_positions[0]);
    }

    // If there's exactly one modified vowel (â, ê, ô, ơ, ư, ă), tone goes on it.
    // Multiple modified vowels (e.g. ươ) fall through to cluster rules.
    let modified: Vec<usize> = vowel_positions
        .iter()
        .copied()
        .filter(|&pos| is_modified_vowel(buf[pos]))
        .collect();
    if modified.len() == 1 {
        return Some(modified[0]);
    }

    // For 3-vowel clusters like "uye", "oai", "uoi" - tone on the second vowel
    if vowel_positions.len() >= 3 {
        return Some(vowel_positions[1]);
    }

    // 2-vowel clusters: closed syllable → second vowel, open → first
    let last_vowel_idx = *vowel_positions.last().unwrap();
    let has_final_consonant =
        buf.len() > last_vowel_idx + 1 && !is_vowel(buf[last_vowel_idx + 1]);
    if has_final_consonant {
        Some(vowel_positions[1])
    } else {
        Some(vowel_positions[0])
    }
}

pub fn apply_tone_to_buffer(buf: &mut [char], tone: Tone) -> bool {
    if let Some(pos) = tone_position(buf) {
        buf[pos] = apply_tone(buf[pos], tone);
        return true;
    }
    false
}

pub fn remove_tone_from_buffer(buf: &mut [char]) -> bool {
    apply_tone_to_buffer(buf, Tone::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- is_vowel --

    #[test]
    fn plain_vowels() {
        for c in ['a', 'e', 'i', 'o', 'u', 'y', 'A', 'E', 'I', 'O', 'U', 'Y'] {
            assert!(is_vowel(c), "{c} should be a vowel");
        }
    }

    #[test]
    fn toned_vowels_are_vowels() {
        for c in ['á', 'à', 'ả', 'ã', 'ạ', 'ế', 'ồ', 'ự', 'ỳ'] {
            assert!(is_vowel(c), "{c} should be a vowel");
        }
    }

    #[test]
    fn consonants_not_vowels() {
        for c in ['b', 'c', 'd', 'g', 'h', 'k', 'l', 'm', 'n', 'đ'] {
            assert!(!is_vowel(c), "{c} should not be a vowel");
        }
    }

    // -- is_modified_vowel --

    #[test]
    fn modified_vowels() {
        for c in ['ă', 'â', 'ê', 'ô', 'ơ', 'ư', 'Ă', 'Â', 'Ê', 'Ô', 'Ơ', 'Ư'] {
            assert!(is_modified_vowel(c), "{c} should be modified");
        }
    }

    #[test]
    fn toned_modified_vowels() {
        for c in ['ắ', 'ầ', 'ễ', 'ồ', 'ở', 'ự'] {
            assert!(is_modified_vowel(c), "{c} should be modified");
        }
    }

    #[test]
    fn plain_vowels_not_modified() {
        for c in ['a', 'e', 'i', 'o', 'u', 'y'] {
            assert!(!is_modified_vowel(c), "{c} should not be modified");
        }
    }

    // -- modify_vowel --

    #[test]
    fn modify_all_pairs() {
        assert_eq!(modify_vowel('a', 'a'), Some('â'));
        assert_eq!(modify_vowel('a', 'w'), Some('ă'));
        assert_eq!(modify_vowel('e', 'e'), Some('ê'));
        assert_eq!(modify_vowel('o', 'o'), Some('ô'));
        assert_eq!(modify_vowel('o', 'w'), Some('ơ'));
        assert_eq!(modify_vowel('u', 'w'), Some('ư'));
    }

    #[test]
    fn modify_preserves_tone() {
        assert_eq!(modify_vowel('á', 'a'), Some('ấ'));
        assert_eq!(modify_vowel('è', 'e'), Some('ề'));
    }

    #[test]
    fn modify_preserves_case() {
        assert_eq!(modify_vowel('A', 'a'), Some('Â'));
        assert_eq!(modify_vowel('O', 'w'), Some('Ơ'));
        assert_eq!(modify_vowel('U', 'w'), Some('Ư'));
    }

    #[test]
    fn modify_invalid_pairs_return_none() {
        assert_eq!(modify_vowel('i', 'i'), None);
        assert_eq!(modify_vowel('u', 'u'), None);
        assert_eq!(modify_vowel('b', 'a'), None);
        assert_eq!(modify_vowel('a', 'e'), None);
    }

    // -- tone_position --

    #[test]
    fn no_vowels() {
        let buf: Vec<char> = "bcd".chars().collect();
        assert_eq!(tone_position(&buf), None);
    }

    #[test]
    fn single_vowel() {
        let buf: Vec<char> = "ba".chars().collect();
        assert_eq!(tone_position(&buf), Some(1));
    }

    #[test]
    fn modified_vowel_preferred() {
        let buf: Vec<char> = "thân".chars().collect();
        assert_eq!(tone_position(&buf), Some(2));
    }

    #[test]
    fn two_vowels_open_syllable() {
        let buf: Vec<char> = "hoa".chars().collect();
        assert_eq!(tone_position(&buf), Some(1)); // 'o'
    }

    #[test]
    fn two_vowels_closed_syllable() {
        let buf: Vec<char> = "toan".chars().collect();
        assert_eq!(tone_position(&buf), Some(2)); // 'a'
    }

    #[test]
    fn three_vowels_second() {
        let buf: Vec<char> = vec!['n', 'g', 'ư', 'ơ', 'i'];
        assert_eq!(tone_position(&buf), Some(3)); // 'ơ'
    }

    // -- apply_tone_to_buffer / remove_tone_from_buffer --

    #[test]
    fn apply_and_remove_buffer() {
        let mut buf: Vec<char> = "ba".chars().collect();
        assert!(apply_tone_to_buffer(&mut buf, Tone::Sac));
        assert_eq!(buf, vec!['b', 'á']);
        assert!(remove_tone_from_buffer(&mut buf));
        assert_eq!(buf, vec!['b', 'a']);
    }

    #[test]
    fn apply_tone_no_vowel_returns_false() {
        let mut buf: Vec<char> = "bc".chars().collect();
        assert!(!apply_tone_to_buffer(&mut buf, Tone::Sac));
    }
}
