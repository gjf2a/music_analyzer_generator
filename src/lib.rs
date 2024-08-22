use std::fmt::Display;

use enum_iterator::Sequence;
use midi_msg::MidiMsg;
use midi_note_recorder::{note_velocity_from, Recording};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence)]
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
}

const MAJOR_ROOT_IDS: [(NoteLetter, Accidental); 12] = [
    (NoteLetter::C, Accidental::Natural),
    (NoteLetter::D, Accidental::Flat),
    (NoteLetter::D, Accidental::Natural),
    (NoteLetter::E, Accidental::Flat),
    (NoteLetter::E, Accidental::Natural),
    (NoteLetter::F, Accidental::Natural),
    (NoteLetter::F, Accidental::Sharp),
    (NoteLetter::G, Accidental::Natural),
    (NoteLetter::A, Accidental::Flat),
    (NoteLetter::A, Accidental::Natural),
    (NoteLetter::B, Accidental::Flat),
    (NoteLetter::B, Accidental::Natural),
];

const MINOR_ROOT_IDS: [(NoteLetter, Accidental); 12] = [
    (NoteLetter::C, Accidental::Natural),
    (NoteLetter::C, Accidental::Sharp),
    (NoteLetter::D, Accidental::Natural),
    (NoteLetter::E, Accidental::Flat),
    (NoteLetter::E, Accidental::Natural),
    (NoteLetter::F, Accidental::Natural),
    (NoteLetter::F, Accidental::Sharp),
    (NoteLetter::G, Accidental::Natural),
    (NoteLetter::G, Accidental::Sharp),
    (NoteLetter::A, Accidental::Natural),
    (NoteLetter::B, Accidental::Flat),
    (NoteLetter::B, Accidental::Natural),
];

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Accidental {
    Flat,
    Natural,
    Sharp,
}

impl Accidental {
    pub fn symbol(&self) -> char {
        match self {
            Accidental::Flat => '\u{266d}',
            Accidental::Natural => '\u{266e}',
            Accidental::Sharp => '\u{266f}',
        }
    }

    pub fn pitch_shift(&self, natural: u8) -> Option<u8> {
        match self {
            Accidental::Flat => {
                if natural > 0 {
                    Some(natural - 1)
                } else {
                    None
                }
            }
            Accidental::Natural => Some(natural),
            Accidental::Sharp => {
                if natural < u8::MAX {
                    Some(natural + 1)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Chord {
    name: ChordName,
    notes: ActivePitches,
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

impl Chord {
    pub fn chords_from(recording: &Recording) -> Vec<(f64, Self)> {
        ActivePitches::pitch_sequence_from(recording)
            .iter()
            .filter_map(|(t, notes)| {
                ChordName::new(*notes).map(|name| {
                    (
                        *t,
                        Self {
                            name,
                            notes: *notes,
                        },
                    )
                })
            })
            .collect()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ChordName {
    note: NoteLetter,
    accidental: Accidental,
    mode: ChordMode,
}

impl Display for ChordName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}{} {:?}",
            self.note,
            self.accidental.symbol(),
            self.mode
        )
    }
}

impl ChordName {
    pub fn new(active: ActivePitches) -> Option<Self> {
        SimpleChordInfo::new(active).map(|info| info.mode())
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
                note,
                accidental,
                mode: if second == 3 {
                    ChordMode::Diminished
                } else {
                    ChordMode::Minor
                },
            }
        } else {
            let (note, accidental) = MAJOR_ROOT_IDS[self.root_pitch_index()];
            ChordName {
                note,
                accidental,
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
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct ActivePitches {
    on: u128,
}

impl ActivePitches {
    pub fn pitch_sequence_from(recording: &Recording) -> Vec<(f64, Self)> {
        let mut current = Self::default();
        let mut result = Vec::new();
        let mut queue = recording.midi_queue();
        while let Some((time, msg)) = queue.pop_front() {
            current.update_from(&msg);
            result.push((time, current));
        }
        result
    }

    pub fn update_from(&mut self, msg: &MidiMsg) {
        if let Some((pitch, velocity)) = note_velocity_from(msg) {
            if velocity > 0 {
                self.on |= 1 << pitch;
            } else {
                self.on &= !(1 << pitch);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.on.count_ones() as usize
    }

    pub fn is_active(&self, pitch: u8) -> bool {
        self.on & (1 << pitch) != 0
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..=127).filter(|p| self.on & (1 << p) > 0)
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct ReducedPitches {
    on: u16,
}

impl ReducedPitches {
    pub fn new(active: ActivePitches) -> Self {
        let mut result = Self::default();
        for pitch in active.iter() {
            result.on |= 1 << (pitch % 12);
        }
        result
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..12).filter(|p| self.on & (1 << p) > 0)
    }

    pub fn pitches_diffs(&self) -> (Vec<u8>, Vec<u8>) {
        let mut diffs = Vec::new();
        let pitches = self.iter().collect::<Vec<_>>();
        for i in 0..pitches.len() {
            let next_pitch = if i + 1 < pitches.len() {
                pitches[i + 1]
            } else {
                12 + pitches[(i + 1) % pitches.len()]
            };
            diffs.push(next_pitch - pitches[i]);
        }
        (pitches, diffs)
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
    use std::collections::BTreeSet;

    use midi_msg::Channel;
    use midi_note_recorder::{midi_msg_from, Recording};
    use rand::Rng;

    use crate::{ActivePitches, Chord};

    #[test]
    fn test_active_pitches() {
        let mut rng = rand::thread_rng();
        let mut active = ActivePitches::default();
        let mut active_tester = BTreeSet::new();
        for _ in 0..100 {
            if active.len() == 0 || rng.gen_bool(0.5) {
                let note = rng.gen_range(0..=127);
                let already = active.is_active(note);
                let msg = midi_msg_from(Channel::Ch1, note, 1);
                let prev_len = active.len();
                active.update_from(&msg);
                assert!(
                    already && active.len() == prev_len || !already && active.len() == prev_len + 1
                );
                assert!(active.is_active(note));
                active_tester.insert(note);
            } else {
                let pitches = active.iter().collect::<Vec<_>>();
                let remove = pitches[rng.gen_range(0..pitches.len())];
                let msg = midi_msg_from(Channel::Ch1, remove, 0);
                let prev_len = active.len();
                active.update_from(&msg);
                assert!(!active.is_active(remove));
                assert_eq!(prev_len - 1, active.len());
                active_tester.remove(&remove);
            }
            let comp = active_tester.iter().copied().collect::<Vec<_>>();
            assert_eq!(comp, active.iter().collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_chord_id() {
        let recording = Recording::from_file("healing4").unwrap();
        let expected = "A♮ Major ([61, 64, 69])
A♮ Major ([61, 64, 69])
B♮ Major ([59, 63, 66])
E♭ Minor ([58, 59, 63, 66])
B♮ Major ([59, 63, 66])
B♮ Major ([59, 63, 66])
B♮ Major ([59, 63, 66])
E♮ Major ([59, 64, 68])
E♮ Major ([59, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
A♮ Major ([61, 64, 69])
A♮ Major ([61, 64, 69])
B♮ Major ([59, 63, 66])
B♮ Major ([59, 63, 66])
E♮ Major ([59, 64, 68])
E♮ Major ([59, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
A♮ Major ([61, 64, 69])
A♮ Major ([61, 64, 69])
B♮ Major ([59, 63, 66])
B♮ Major ([59, 63, 66])
B♮ Major ([59, 63, 66])
E♮ Major ([59, 64, 68])
E♮ Major ([59, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
A♮ Major ([57, 61, 64])
B♮ Major ([59, 63, 66])
E♭ Diminished ([57, 59, 63, 66])
B♮ Major ([59, 63, 66])
B♮ Major ([59, 63, 66])";
        let chords = Chord::chords_from(&recording);
        for (i, chord_str) in expected.lines().enumerate() {
            assert_eq!(format!("{}", chords[i].1), chord_str);
        }
    }
}
