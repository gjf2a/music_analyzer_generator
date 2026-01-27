use std::fmt::Display;

use enum_iterator::Sequence;

use crate::MAJOR_ROOT_IDS;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence, Hash)]
pub enum NoteLetter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl NoteLetter {
    pub fn next(&self) -> Self {
        enum_iterator::next_cycle(self)
    }

    pub fn prev(&self) -> Self {
        enum_iterator::previous_cycle(self)
    }

    pub fn natural_pitch(&self) -> u8 {
        match self {
            NoteLetter::C => 0,
            NoteLetter::D => 2,
            NoteLetter::E => 4,
            NoteLetter::F => 5,
            NoteLetter::G => 7,
            NoteLetter::A => 9,
            NoteLetter::B => 11,
        }
    }

    pub fn steps_above_natural(&self, pitch: u8) -> i16 {
        let mut pitch = pitch;
        while pitch < self.natural_pitch() {
            pitch += 12;
        }
        let start = self.natural_pitch();
        let note_offset = ((pitch - start) % 12) as i16;
        if note_offset > 6 {
            note_offset - 12
        } else {
            note_offset
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Accidental {
    DoubleFlat,
    Flat,
    Natural,
    Sharp,
    DoubleSharp,
}

impl Accidental {
    pub fn symbol(&self) -> char {
        match self {
            Self::DoubleFlat => '\u{1d12b}',
            Self::Flat => '\u{266d}',
            //Accidental::Natural => '\u{266e}',
            Self::Natural => ' ',
            Self::Sharp => '\u{266f}',
            Self::DoubleSharp => '\u{1d12a}',
        }
    }

    pub fn sharpen(&self) -> Option<Self> {
        match self {
            Self::DoubleFlat => Some(Self::Flat),
            Self::Flat => Some(Self::Natural),
            Self::Natural => Some(Self::Sharp),
            Self::Sharp => Some(Self::DoubleSharp),
            Self::DoubleSharp => None,
        }
    }

    pub fn flatten(&self) -> Option<Self> {
        match self {
            Self::DoubleFlat => None,
            Self::Flat => Some(Self::DoubleFlat),
            Self::Natural => Some(Self::Flat),
            Self::Sharp => Some(Self::Natural),
            Self::DoubleSharp => Some(Self::Sharp),
        }
    }

    pub fn offset_value(&self) -> i16 {
        match self {
            Self::DoubleFlat => -2,
            Self::Flat => -1,
            Self::Natural => 0,
            Self::Sharp => 1,
            Self::DoubleSharp => 2,
        }
    }

    pub fn pitch_shift(&self, natural: u8) -> Option<u8> {
        match self {
            Self::DoubleFlat => {
                if natural > 1 {
                    Some(natural - 2)
                } else {
                    None
                }
            }
            Self::Flat => {
                if natural > 0 {
                    Some(natural - 1)
                } else {
                    None
                }
            }
            Self::Natural => Some(natural),
            Self::Sharp => {
                if natural < u8::MAX {
                    Some(natural + 1)
                } else {
                    None
                }
            }
            Self::DoubleSharp => {
                if natural + 1 < u8::MAX {
                    Some(natural + 2)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct NoteName {
    ltr: NoteLetter,
    acc: Accidental,
}

impl NoteName {
    pub fn new(ltr: NoteLetter, acc: Accidental) -> Self {
        Self { ltr, acc }
    }

    pub fn letter(&self) -> NoteLetter {
        self.ltr
    }

    pub fn accidental(&self) -> Accidental {
        self.acc
    }

    pub fn flatten(&mut self) {
        if let Some(flattened) = self.acc.flatten() {
            self.acc = flattened;
        } else {
            panic!("Can't flatten {:?}", self.acc);
        }
    }

    pub fn sharpen(&mut self) {
        if let Some(sharpened) = self.acc.sharpen() {
            self.acc = sharpened;
        } else {
            panic!("Can't sharpen {:?}", self.acc);
        }
    }

    pub fn name_of(pitch: u8) -> Self {
        let (letter, modifier) = MAJOR_ROOT_IDS[(pitch % 12) as usize];
        Self {
            ltr: letter,
            acc: modifier,
        }
    }

    pub fn reference_pitch(&self) -> Option<u8> {
        self.acc.pitch_shift(self.ltr.natural_pitch())
    }

    pub fn synonym_of(&self, other: &NoteName) -> bool {
        if let (Some(p1), Some(p2)) = (self.reference_pitch(), other.reference_pitch()) {
            p1 == p2
        } else {
            false
        }
    }

    pub fn full_name_for(letter: NoteLetter, pitch: u8) -> Self {
        let offset = letter.steps_above_natural(pitch);
        Self {
            ltr: letter,
            acc: match offset {
                -2 => Accidental::DoubleFlat,
                -1 => Accidental::Flat,
                0 => Accidental::Natural,
                1 => Accidental::Sharp,
                2 => Accidental::DoubleSharp,
                _ => panic!("Offset {offset} beyond +/- 2 undefined"),
            },
        }
    }

    pub fn lowest_midi_note(&self) -> u8 {
        self.acc.pitch_shift(self.ltr.natural_pitch()).unwrap()
    }
}

impl Display for NoteName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}{}", self.ltr, self.acc.symbol(),)
    }
}

#[cfg(test)]
mod tests {
    use crate::notes::Accidental::Flat as F;
    use crate::notes::Accidental::Natural as N;
    use crate::notes::Accidental::Sharp as S;
    use crate::notes::NoteLetter as NL;
    use crate::notes::NoteName;

    #[test]
    fn test_note_names() {
        let note_name = [
            (60, NL::C, N),
            (61, NL::D, F),
            (62, NL::D, N),
            (63, NL::E, F),
            (64, NL::E, N),
            (65, NL::F, N),
            (66, NL::F, S),
            (67, NL::G, N),
            (68, NL::A, F),
            (69, NL::A, N),
            (70, NL::B, F),
            (71, NL::B, N),
        ];
        for (pitch, ltr, acc) in note_name {
            assert_eq!(NoteName::name_of(pitch), NoteName::new(ltr, acc));
        }
    }
}
