#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    None,
    Sac,    // acute ´
    Huyen,  // grave `
    Hoi,    // hook ̉
    Nga,    // tilde ~
    Nang,   // dot .
}

impl Tone {
    pub fn from_telex(c: char) -> Option<Tone> {
        match c {
            's' | 'S' => Some(Tone::Sac),
            'f' | 'F' => Some(Tone::Huyen),
            'r' | 'R' => Some(Tone::Hoi),
            'x' | 'X' => Some(Tone::Nga),
            'j' | 'J' => Some(Tone::Nang),
            'z' | 'Z' => Some(Tone::None),
            _ => None,
        }
    }
}

pub fn apply_tone(base: char, tone: Tone) -> char {
    let stripped = strip_tone(base);
    let row = match TONE_TABLE.iter().position(|r| r[0] == stripped) {
        Some(i) => i,
        None => return base,
    };
    let col = match tone {
        Tone::None => 0,
        Tone::Sac => 1,
        Tone::Huyen => 2,
        Tone::Hoi => 3,
        Tone::Nga => 4,
        Tone::Nang => 5,
    };
    TONE_TABLE[row][col]
}

pub fn strip_tone(c: char) -> char {
    for row in TONE_TABLE {
        if row.iter().any(|&x| x == c) {
            return row[0];
        }
    }
    c
}

pub fn get_tone(c: char) -> Tone {
    for row in TONE_TABLE {
        if let Some(col) = row.iter().position(|&x| x == c) {
            return match col {
                1 => Tone::Sac,
                2 => Tone::Huyen,
                3 => Tone::Hoi,
                4 => Tone::Nga,
                5 => Tone::Nang,
                _ => Tone::None,
            };
        }
    }
    Tone::None
}

// [base, sac, huyen, hoi, nga, nang]
const TONE_TABLE: &[[char; 6]] = &[
    ['a', 'á', 'à', 'ả', 'ã', 'ạ'],
    ['ă', 'ắ', 'ằ', 'ẳ', 'ẵ', 'ặ'],
    ['â', 'ấ', 'ầ', 'ẩ', 'ẫ', 'ậ'],
    ['e', 'é', 'è', 'ẻ', 'ẽ', 'ẹ'],
    ['ê', 'ế', 'ề', 'ể', 'ễ', 'ệ'],
    ['i', 'í', 'ì', 'ỉ', 'ĩ', 'ị'],
    ['o', 'ó', 'ò', 'ỏ', 'õ', 'ọ'],
    ['ô', 'ố', 'ồ', 'ổ', 'ỗ', 'ộ'],
    ['ơ', 'ớ', 'ờ', 'ở', 'ỡ', 'ợ'],
    ['u', 'ú', 'ù', 'ủ', 'ũ', 'ụ'],
    ['ư', 'ứ', 'ừ', 'ử', 'ữ', 'ự'],
    ['y', 'ý', 'ỳ', 'ỷ', 'ỹ', 'ỵ'],
    ['A', 'Á', 'À', 'Ả', 'Ã', 'Ạ'],
    ['Ă', 'Ắ', 'Ằ', 'Ẳ', 'Ẵ', 'Ặ'],
    ['Â', 'Ấ', 'Ầ', 'Ẩ', 'Ẫ', 'Ậ'],
    ['E', 'É', 'È', 'Ẻ', 'Ẽ', 'Ẹ'],
    ['Ê', 'Ế', 'Ề', 'Ể', 'Ễ', 'Ệ'],
    ['I', 'Í', 'Ì', 'Ỉ', 'Ĩ', 'Ị'],
    ['O', 'Ó', 'Ò', 'Ỏ', 'Õ', 'Ọ'],
    ['Ô', 'Ố', 'Ồ', 'Ổ', 'Ỗ', 'Ộ'],
    ['Ơ', 'Ớ', 'Ờ', 'Ở', 'Ỡ', 'Ợ'],
    ['U', 'Ú', 'Ù', 'Ủ', 'Ũ', 'Ụ'],
    ['Ư', 'Ứ', 'Ừ', 'Ử', 'Ữ', 'Ự'],
    ['Y', 'Ý', 'Ỳ', 'Ỷ', 'Ỹ', 'Ỵ'],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_telex_maps_all_keys() {
        assert_eq!(Tone::from_telex('s'), Some(Tone::Sac));
        assert_eq!(Tone::from_telex('f'), Some(Tone::Huyen));
        assert_eq!(Tone::from_telex('r'), Some(Tone::Hoi));
        assert_eq!(Tone::from_telex('x'), Some(Tone::Nga));
        assert_eq!(Tone::from_telex('j'), Some(Tone::Nang));
        assert_eq!(Tone::from_telex('z'), Some(Tone::None));
        assert_eq!(Tone::from_telex('a'), None);
        assert_eq!(Tone::from_telex('1'), None);
    }

    #[test]
    fn from_telex_uppercase() {
        assert_eq!(Tone::from_telex('S'), Some(Tone::Sac));
        assert_eq!(Tone::from_telex('Z'), Some(Tone::None));
    }

    #[test]
    fn apply_tone_all_variants() {
        assert_eq!(apply_tone('a', Tone::Sac), 'á');
        assert_eq!(apply_tone('a', Tone::Huyen), 'à');
        assert_eq!(apply_tone('a', Tone::Hoi), 'ả');
        assert_eq!(apply_tone('a', Tone::Nga), 'ã');
        assert_eq!(apply_tone('a', Tone::Nang), 'ạ');
        assert_eq!(apply_tone('a', Tone::None), 'a');
    }

    #[test]
    fn apply_tone_on_modified_vowels() {
        assert_eq!(apply_tone('â', Tone::Sac), 'ấ');
        assert_eq!(apply_tone('ê', Tone::Huyen), 'ề');
        assert_eq!(apply_tone('ơ', Tone::Hoi), 'ở');
        assert_eq!(apply_tone('ư', Tone::Nang), 'ự');
    }

    #[test]
    fn apply_tone_uppercase() {
        assert_eq!(apply_tone('A', Tone::Sac), 'Á');
        assert_eq!(apply_tone('Ê', Tone::Huyen), 'Ề');
    }

    #[test]
    fn apply_tone_replaces_existing() {
        assert_eq!(apply_tone('á', Tone::Huyen), 'à');
        assert_eq!(apply_tone('ề', Tone::Sac), 'ế');
    }

    #[test]
    fn apply_tone_non_vowel_passthrough() {
        assert_eq!(apply_tone('b', Tone::Sac), 'b');
        assert_eq!(apply_tone('1', Tone::Huyen), '1');
    }

    #[test]
    fn strip_tone_removes_all() {
        assert_eq!(strip_tone('á'), 'a');
        assert_eq!(strip_tone('ả'), 'a');
        assert_eq!(strip_tone('ạ'), 'a');
        assert_eq!(strip_tone('ề'), 'ê');
        assert_eq!(strip_tone('ự'), 'ư');
    }

    #[test]
    fn strip_tone_no_tone_unchanged() {
        assert_eq!(strip_tone('a'), 'a');
        assert_eq!(strip_tone('â'), 'â');
        assert_eq!(strip_tone('b'), 'b');
    }

    #[test]
    fn get_tone_identifies_correctly() {
        assert_eq!(get_tone('á'), Tone::Sac);
        assert_eq!(get_tone('à'), Tone::Huyen);
        assert_eq!(get_tone('ả'), Tone::Hoi);
        assert_eq!(get_tone('ã'), Tone::Nga);
        assert_eq!(get_tone('ạ'), Tone::Nang);
        assert_eq!(get_tone('a'), Tone::None);
        assert_eq!(get_tone('b'), Tone::None);
    }

    #[test]
    fn roundtrip_apply_then_strip() {
        for base in ['a', 'ă', 'â', 'e', 'ê', 'o', 'ô', 'ơ', 'u', 'ư', 'y'] {
            for tone in [Tone::Sac, Tone::Huyen, Tone::Hoi, Tone::Nga, Tone::Nang] {
                let toned = apply_tone(base, tone);
                assert_eq!(strip_tone(toned), base, "strip(apply({base}, {tone:?}))");
                assert_eq!(get_tone(toned), tone, "get_tone(apply({base}, {tone:?}))");
            }
        }
    }
}
