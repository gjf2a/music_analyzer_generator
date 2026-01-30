use std::fmt::Display;

use enum_iterator::Sequence;

use crate::{
    ActivePitches, MAJOR_ROOT_IDS, MINOR_ROOT_IDS, ReducedPitches,
    notes::{Accidental, NoteLetter, NoteName},
    scales::{RootedScale, ScaleMode},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Chord {
    name: ChordName,
    notes: ActivePitches,
}

impl Chord {
    pub fn new(name: ChordName, notes: ActivePitches) -> Self {
        Self { name, notes }
    }

    pub fn name(&self) -> ChordName {
        self.name
    }
}

impl Display for Chord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({:?})",
            self.name,
            self.notes.iter().collect::<Vec<_>>()
        )
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ChordName {
    letter: NoteLetter,
    modifier: Accidental,
    mode: ChordMode,
}

impl Display for ChordName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}{} {:?}",
            self.letter,
            self.modifier.symbol(),
            self.mode
        )
    }
}

impl From<ActivePitches> for Option<ChordName> {
    fn from(value: ActivePitches) -> Self {
        SimpleChordInfo::new(value).map(|info| info.mode())
    }
}

impl ChordName {
    pub fn new(letter: NoteLetter, modifier: Accidental, mode: ChordMode) -> Self {
        Self {
            letter,
            modifier,
            mode,
        }
    }

    pub fn missing_chord_tones_from(&self, scale: &RootedScale) -> Vec<NoteName> {
        self.note_names()
            .iter()
            .filter(|chord_note| {
                scale
                    .all_diatonic_note_names_unordered()
                    .all(|scale_note| !chord_note.synonym_of(&scale_note))
            })
            .copied()
            .collect()
    }

    pub fn root_name(&self) -> NoteName {
        NoteName::new(self.letter, self.modifier)
    }

    pub fn note_names(&self) -> Vec<NoteName> {
        let rs = self.mode.note_name_scale().rooted(self.root_name());
        let mut result = rs
            .all_diatonic_notes_up()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, (_, n))| n)
            .take(self.mode.num_chord_notes())
            .collect::<Vec<_>>();
        if self.mode == ChordMode::Diminished7 {
            let end = result.len() - 1;
            result[end].flatten();
        }
        result
    }

    pub fn compact_name(&self) -> String {
        let base_note_letter = format!("{:?}{}", self.letter, self.modifier.symbol());
        let note_letter = base_note_letter.trim();
        match self.mode {
            ChordMode::Major => note_letter.to_owned(),
            ChordMode::Minor => format!("{note_letter}m"),
            ChordMode::Diminished => format!("{note_letter}\u{00b0}"),
            ChordMode::Augmented => format!("{note_letter}+"),
            ChordMode::Dominant7 => format!("{note_letter}7"),
            ChordMode::Major7 => format!("{note_letter}maj7"),
            ChordMode::Minor7 => format!("{note_letter}m7"),
            ChordMode::Diminished7 => format!("{note_letter}\u{00b0}7"),
            ChordMode::HalfDiminished7 => format!("{note_letter}\u{00f8}7"),
        }
    }
}

struct SimpleChordInfo {
    pitches: Vec<u8>,
    root_index: usize,
    thirds: Vec<u8>,
}

impl SimpleChordInfo {
    fn new(active: ActivePitches) -> Option<Self> {
        let (pitches, diffs) = ReducedPitches::new(active).pitches_diffs();
        first_third_index(&diffs).map(|root_index| {
            let thirds = (0..diffs.len())
                .map(|i| diffs[(root_index + i) % diffs.len()])
                .collect();
            Self {
                pitches,
                root_index,
                thirds,
            }
        })
    }

    fn root_pitch_index(&self) -> usize {
        self.pitches[self.root_index] as usize
    }

    fn mode(&self) -> ChordName {
        let first = self.thirds[0];
        let second = self.thirds[1];
        if first == 3 {
            let (note, accidental) = MINOR_ROOT_IDS[self.root_pitch_index()];
            ChordName {
                letter: note,
                modifier: accidental,
                mode: if second == 3 {
                    ChordMode::Diminished
                } else {
                    ChordMode::Minor
                },
            }
        } else {
            let (note, accidental) = MAJOR_ROOT_IDS[self.root_pitch_index()];
            ChordName {
                letter: note,
                modifier: accidental,
                mode: if second == 3 {
                    ChordMode::Major
                } else {
                    ChordMode::Augmented
                },
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence)]
pub enum ChordMode {
    Major,
    Minor,
    Diminished,
    Augmented,
    Dominant7,
    Major7,
    Minor7,
    Diminished7,
    HalfDiminished7,
}

impl ChordMode {
    fn note_name_scale(&self) -> ScaleMode {
        match self {
            Self::Major => ScaleMode::Major,
            Self::Minor => ScaleMode::Minor,
            Self::Diminished => ScaleMode::Locrian,
            Self::Augmented => ScaleMode::WholeTone,
            Self::Dominant7 => ScaleMode::Mixolydian,
            Self::Major7 => ScaleMode::Major,
            Self::Minor7 => ScaleMode::Minor,
            Self::Diminished7 => ScaleMode::Locrian,
            Self::HalfDiminished7 => ScaleMode::Locrian,
        }
    }

    fn num_chord_notes(&self) -> usize {
        match self {
            Self::Major | Self::Minor | Self::Diminished | Self::Augmented => 3,
            Self::Dominant7
            | Self::Major7
            | Self::Minor7
            | Self::Diminished7
            | Self::HalfDiminished7 => 4,
        }
    }
}

fn major_or_minor_third(interval: u8) -> bool {
    interval == 3 || interval == 4
}

fn first_third_index(diffs: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < diffs.len() {
        if major_or_minor_third(diffs[i]) && major_or_minor_third(diffs[(i + 1) % diffs.len()]) {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::chords::ChordMode as CM;
    use crate::chords::ChordName;
    use crate::notes::Accidental::Flat as F;
    use crate::notes::Accidental::Natural as N;
    use crate::notes::Accidental::Sharp as S;
    use crate::notes::NoteLetter as NL;
    use crate::notes::NoteName;

    #[test]
    fn test_chord_notes() {
        for (letter, modifier, mode, notes) in [
            (
                NL::C,
                N,
                CM::Major,
                vec![(NL::C, N), (NL::E, N), (NL::G, N)],
            ),
            (
                NL::D,
                N,
                CM::Major,
                vec![(NL::D, N), (NL::F, S), (NL::A, N)],
            ),
            (
                NL::E,
                F,
                CM::Major,
                vec![(NL::E, F), (NL::G, N), (NL::B, F)],
            ),
            (
                NL::G,
                N,
                CM::Dominant7,
                vec![(NL::G, N), (NL::B, N), (NL::D, N), (NL::F, N)],
            ),
            (
                NL::G,
                N,
                CM::Minor,
                vec![(NL::G, N), (NL::B, F), (NL::D, N)],
            ),
            (
                NL::G,
                N,
                CM::Minor7,
                vec![(NL::G, N), (NL::B, F), (NL::D, N), (NL::F, N)],
            ),
            (
                NL::G,
                N,
                CM::HalfDiminished7,
                vec![(NL::G, N), (NL::B, F), (NL::D, F), (NL::F, N)],
            ),
            (
                NL::G,
                S,
                CM::Diminished7,
                vec![(NL::G, S), (NL::B, N), (NL::D, N), (NL::F, N)],
            ),
            (
                NL::G,
                N,
                CM::Diminished7,
                vec![(NL::G, N), (NL::B, F), (NL::D, F), (NL::F, F)],
            ),
        ] {
            let chord_name = ChordName::new(letter, modifier, mode);
            let expected = notes
                .iter()
                .copied()
                .map(|(letter, modifier)| NoteName::new(letter, modifier))
                .collect::<Vec<_>>();
            assert_eq!(expected, chord_name.note_names());
        }
    }
}
