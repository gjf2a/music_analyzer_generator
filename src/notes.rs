use std::{fmt::Display, ops::Add};

use enum_iterator::Sequence;
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};

use crate::{MAJOR_ROOT_IDS, NoteDuration};

pub fn octave_equivalent(p1: u8, p2: u8) -> bool {
    p1 % 12 == p2 % 12
}

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

impl TryFrom<i16> for Accidental {
    type Error = anyhow::Error;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            -2 => Ok(Self::DoubleFlat),
            -1 => Ok(Self::Flat),
            0 => Ok(Self::Natural),
            1 => Ok(Self::Sharp),
            2 => Ok(Self::DoubleSharp),
            _ => Err(anyhow::anyhow!(
                "{value} cannot be converted to an Accidental"
            )),
        }
    }
}

impl Add for Accidental {
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        Self::try_from(self.offset_value() + rhs.offset_value()).ok()
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

    pub fn with_acc(&self, acc: Accidental) -> Self {
        Self::new(self.ltr, acc)
    }

    pub fn modified(&self, modifier: Accidental) -> Option<Self> {
        (self.acc + modifier).map(|acc| Self { ltr: self.ltr, acc })
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

    pub fn is_pitch_match(&self, pitch: u8) -> bool {
        self.reference_pitch()
            .map_or(false, |p| p % 12 == pitch % 12)
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

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Note {
    pitch: u8,
    velocity: u8,
    duration: NoteDuration,
}

impl Note {
    pub fn new(pitch: u8, velocity: u8) -> Self {
        Self {
            pitch,
            velocity,
            duration: 0.0,
        }
    }

    pub fn octave_equivalent(&self, other: Self) -> bool {
        octave_equivalent(self.pitch, other.pitch)
            && self.velocity == other.velocity
            && self.duration == other.duration
    }

    pub fn repitched(&self, repitch: u8) -> Self {
        Self {
            pitch: repitch,
            velocity: self.velocity,
            duration: self.duration,
        }
    }

    pub fn is_rest(&self) -> bool {
        self.velocity == 0
    }

    pub fn pitch(&self) -> u8 {
        self.pitch
    }

    pub fn velocity(&self) -> u8 {
        self.velocity
    }

    pub fn duration(&self) -> NoteDuration {
        self.duration
    }

    pub fn set_duration(&mut self, new_duration: NoteDuration) {
        self.duration = new_duration;
    }

    pub fn midi_on_off(&self) -> (MidiMsg, MidiMsg) {
        (
            make_midi_msg(self.pitch, self.velocity),
            make_midi_msg(self.pitch, 0),
        )
    }
}

fn make_midi_msg(pitch: u8, velocity: u8) -> MidiMsg {
    let msg = if velocity == 0 {
        ChannelVoiceMsg::NoteOff {
            note: pitch,
            velocity,
        }
    } else {
        ChannelVoiceMsg::NoteOn {
            note: pitch,
            velocity,
        }
    };
    MidiMsg::ChannelVoice {
        channel: Channel::Ch1,
        msg,
    }
}

#[cfg(test)]
mod tests {
    use crate::notes::Accidental::DoubleFlat as DF;
    use crate::notes::Accidental::DoubleSharp as DS;
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

    #[test]
    fn test_pitch_match() {
        for (ltr, acc, candidate, is_match) in [
            (NL::C, N, 60, true),
            (NL::C, N, 59, false),
            (NL::B, S, 60, true),
            (NL::G, F, 66, true),
            (NL::G, N, 66, false),
            (NL::A, F, 69, false),
            (NL::A, N, 69, true),
            (NL::D, S, 63, true),
            (NL::D, S, 62, false),
        ] {
            let name = NoteName::new(ltr, acc);
            assert_eq!(name.is_pitch_match(candidate), is_match);
        }
    }

    #[test]
    fn test_add_accidental() {
        for (a, b, c) in [
            (N, S, Some(S)),
            (S, F, Some(N)),
            (F, S, Some(N)),
            (F, N, Some(F)),
            (F, F, Some(DF)),
            (S, S, Some(DS)),
            (DS, F, Some(S)),
            (DF, F, None),
            (DS, S, None),
            (DS, DS, None),
            (DS, DF, Some(N)),
        ] {
            assert_eq!(a + b, c);
        }
    }

    #[test]
    fn test_modified_note_name() {
        for (ltr, acc, modifier, target) in [
            (NL::A, N, S, S),
            (NL::F, S, S, DS),
            (NL::B, F, S, N),
            (NL::B, F, N, F),
        ] {
            let start = NoteName::new(ltr, acc);
            let target = NoteName::new(ltr, target);
            assert_eq!(start.modified(modifier).unwrap(), target);
        }
    }

    #[test]
    fn test_with_acc() {
        for (ltr, acc, modifier, target) in [
            (NL::A, N, S, S),
            (NL::F, S, S, S),
            (NL::B, F, S, S),
            (NL::B, F, N, N),
        ] {
            let start = NoteName::new(ltr, acc);
            let target = NoteName::new(ltr, target);
            assert_eq!(start.with_acc(modifier), target);
        }
    }
}
