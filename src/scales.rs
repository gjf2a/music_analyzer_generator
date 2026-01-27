use std::collections::{BTreeMap, HashSet};

use bare_metal_modulo::{MNum, ModNum};
use enum_iterator::{Sequence, all};

use crate::notes::{Accidental, NoteLetter, NoteName};

#[derive(Copy, Clone, Debug, Sequence, Eq, PartialEq)]
pub enum ScaleMode {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    WholeTone,
    HarmonicMinor,
    MelodicMinor,
    Diminished,
    Augmented,
}

impl ScaleMode {
    pub fn rooted(&self, root: NoteName) -> RootedScale {
        RootedScale::new(*self, root)
    }

    fn pattern_up(&self) -> ScalePattern {
        match self {
            Self::Major => ScalePattern::mode_rotation(0),
            Self::Dorian => ScalePattern::mode_rotation(1),
            Self::Phrygian => ScalePattern::mode_rotation(2),
            Self::Lydian => ScalePattern::mode_rotation(3),
            Self::Mixolydian => ScalePattern::mode_rotation(4),
            Self::Minor => ScalePattern::mode_rotation(5),
            Self::Locrian => ScalePattern::mode_rotation(6),
            Self::HarmonicMinor => ScalePattern::standard([2, 1, 2, 2, 1, 3, 1]),
            Self::MelodicMinor => ScalePattern::standard([2, 1, 2, 2, 2, 2, 1]),
            Self::WholeTone => ScalePattern {
                num_jumps: 6,
                jumps: [2, 2, 2, 2, 2, 2, 0, 0],
            },
            Self::Diminished => ScalePattern {
                num_jumps: 8,
                jumps: [2, 1, 2, 1, 2, 1, 2, 1],
            },
            Self::Augmented => ScalePattern {
                num_jumps: 6,
                jumps: [3, 1, 3, 1, 3, 1, 0, 0],
            },
        }
    }

    pub fn letter_iterator(&self, root_letter: NoteLetter) -> ScaleLetterIterator {
        let mut letter_seq = all::<NoteLetter>()
            .cycle()
            .skip_while(|nl| *nl != root_letter)
            .take(7)
            .collect::<Vec<_>>();
        match self {
            Self::WholeTone => {
                letter_seq.pop();
            }
            Self::Augmented => {
                letter_seq.remove(5);
                letter_seq[3] = letter_seq[4];
            }
            Self::Diminished => {
                letter_seq.insert(4, letter_seq[4]);
            }
            _ => {}
        }
        ScaleLetterIterator {
            pos: ModNum::new(0, letter_seq.len()),
            letter_seq,
        }
    }

    fn notes_going_up(&self, note: NoteName) -> impl Iterator<Item = u8> {
        ScaleUpIterator {
            pattern: self.pattern_up(),
            current: note.lowest_midi_note(),
            count: 0,
        }
    }

    fn notes_going_down(&self, note: NoteName) -> impl Iterator<Item = u8> {
        let root_note = note.lowest_midi_note();
        ScaleDownIterator {
            pattern: self.pattern_down(),
            current: root_note + if root_note > 7 { 108 } else { 120 },
            count: 0,
        }
    }

    fn pattern_down(&self) -> ScalePattern {
        match self {
            Self::MelodicMinor => ScalePattern::mode_rotation(5),
            _ => self.pattern_up(),
        }
        .reversed()
    }
}

pub struct ScaleLetterIterator {
    pos: ModNum<usize>,
    letter_seq: Vec<NoteLetter>,
}

impl Iterator for ScaleLetterIterator {
    type Item = NoteLetter;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.letter_seq[self.pos.a()];
        self.pos += 1;
        Some(result)
    }
}

impl DoubleEndedIterator for ScaleLetterIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        let result = self.letter_seq[self.pos.a()];
        self.pos -= 1;
        Some(result)
    }
}

#[derive(Debug, Clone)]
pub struct RootedScale {
    mode: ScaleMode,
    root: NoteName,
    notes2letters_up: BTreeMap<u8, NoteLetter>,
    notes2names_up: BTreeMap<u8, NoteName>,
    notes2letters_down: BTreeMap<u8, NoteLetter>,
    notes2names_down: BTreeMap<u8, NoteName>,
    all_note_names: HashSet<NoteName>,
}

impl RootedScale {
    pub fn new(mode: ScaleMode, root: NoteName) -> Self {
        let mut result = Self {
            mode,
            root,
            notes2letters_up: BTreeMap::default(),
            notes2names_up: BTreeMap::default(),
            notes2letters_down: BTreeMap::default(),
            notes2names_down: BTreeMap::default(),
            all_note_names: HashSet::default(),
        };
        let notes2letters_up = result.all_diatonic_note_letters_up().collect();
        result.notes2letters_up = notes2letters_up;
        let notes2names = result.all_diatonic_notes_up().collect();
        result.notes2names_up = notes2names;
        let notes2letters_down = result.all_diatonic_note_letters_down().collect();
        result.notes2letters_down = notes2letters_down;
        let notes2names_down = result.all_diatonic_notes_down().collect();
        result.notes2names_down = notes2names_down;
        for note_name in result.notes2names_up.values() {
            result.all_note_names.insert(*note_name);
        }
        for note_name in result.notes2names_down.values() {
            result.all_note_names.insert(*note_name);
        }
        result
    }

    pub fn root_name(&self) -> NoteName {
        self.root
    }

    pub fn mode(&self) -> ScaleMode {
        self.mode
    }

    pub fn all_diatonic_note_letters_up(&self) -> impl Iterator<Item = (u8, NoteLetter)> {
        self.notes_going_up()
            .zip(self.mode.letter_iterator(self.root.letter()))
    }

    pub fn all_diatonic_notes_up(&self) -> impl Iterator<Item = (u8, NoteName)> {
        self.all_diatonic_note_letters_up()
            .map(|(pitch, letter)| (pitch, NoteName::full_name_for(letter, pitch)))
    }

    pub fn all_diatonic_note_letters_down(&self) -> impl Iterator<Item = (u8, NoteLetter)> {
        self.notes_going_down()
            .zip(self.mode.letter_iterator(self.root.letter()).rev())
    }

    pub fn all_diatonic_notes_down(&self) -> impl Iterator<Item = (u8, NoteName)> {
        self.all_diatonic_note_letters_down()
            .map(|(pitch, letter)| (pitch, NoteName::full_name_for(letter, pitch)))
    }

    pub fn all_diatonic_note_names_unordered(&self) -> impl Iterator<Item = NoteName> {
        self.all_note_names.iter().copied()
    }

    pub fn contains(&self, note: u8) -> bool {
        self.notes2letters_down.contains_key(&note)
    }

    pub fn contains_ascending(&self, note: u8) -> bool {
        self.notes2letters_up.contains_key(&note)
    }

    pub fn name_of(&self, pitch: u8) -> Option<NoteName> {
        self.notes2names_down.get(&pitch).copied()
    }

    pub fn name_of_ascending(&self, pitch: u8) -> Option<NoteName> {
        self.notes2names_up.get(&pitch).copied()
    }

    pub fn middle_c(&self) -> u8 {
        if self.contains(60) { 60 } else { 61 }
    }

    pub fn all_sharps(&self) -> impl Iterator<Item = NoteLetter> {
        self.all_diatonic_notes_down()
            .take(7)
            .filter(|(_, n)| n.accidental() == Accidental::Sharp)
            .map(|(_, n)| n.letter())
    }

    pub fn all_flats(&self) -> impl Iterator<Item = NoteLetter> {
        self.all_diatonic_notes_down()
            .take(7)
            .filter(|(_, n)| n.accidental() == Accidental::Flat)
            .map(|(_, n)| n.letter())
    }

    pub fn round_up(&self, pitch: u8) -> u8 {
        assert!(pitch <= *self.notes2letters_down.last_key_value().unwrap().0);
        let mut rounded = pitch;
        while !self.contains(rounded) {
            rounded += 1;
        }
        rounded
    }

    pub fn round_down(&self, pitch: u8) -> u8 {
        let mut rounded = pitch;
        while !self.contains(rounded) {
            rounded -= 1;
        }
        rounded
    }

    pub fn notes_going_up(&self) -> impl Iterator<Item = u8> {
        self.mode.notes_going_up(self.root)
    }

    pub fn notes_going_down(&self) -> impl Iterator<Item = u8> {
        self.mode.notes_going_down(self.root)
    }

    pub fn note_up(&self, current: u8, interval: usize) -> Option<u8> {
        self.notes_going_up()
            .skip_while(|n| *n < current)
            .skip(interval - 1)
            .next()
    }

    pub fn note_down(&self, current: u8, interval: usize) -> Option<u8> {
        self.notes_going_down()
            .skip_while(|n| *n > current)
            .skip(interval - 1)
            .next()
    }

    pub fn diatonic_bracket_for(&self, pitch: u8) -> Option<(u8, u8)> {
        if self.contains(pitch) {
            None
        } else {
            let below = self.round_down(pitch);
            let above = self.round_up(pitch);
            Some((below, above))
        }
    }

    pub fn diatonic_steps_between(&self, pitch_1: u8, pitch_2: u8) -> Option<u8> {
        if pitch_1 > pitch_2 {
            self.diatonic_steps_between(pitch_2, pitch_1)
        } else {
            let interval = self
                .notes_going_up()
                .skip_while(|n| *n < pitch_1)
                .take_while(|n| *n <= pitch_2)
                .collect::<Vec<_>>();
            if interval.len() == 0
                || interval[0] != pitch_1
                || interval[interval.len() - 1] != pitch_2
            {
                None
            } else {
                Some((interval.len() - 1) as u8)
            }
        }
    }

    pub fn descending_match(&self, pitch: u8) -> (NoteName, u8, Option<Accidental>) {
        if let Some((down, up)) = self.diatonic_bracket_for(pitch) {
            let down_name = self.name_of(down).unwrap();
            let up_name = self.name_of(up).unwrap();
            let down_modifier = down_name.accidental().sharpen();
            let up_modifier = up_name.accidental().flatten();
            if let Some(up_modifier) = up_modifier {
                if up_modifier.offset_value() > -2 {
                    return (up_name, up, Some(up_modifier));
                }
            }
            (down_name, down, down_modifier)
        } else {
            (self.name_of(pitch).unwrap(), pitch, None)
        }
    }

    pub fn ascending_match(&self, pitch: u8) -> (NoteName, u8, Option<Accidental>) {
        if let Some((down, up)) = self.diatonic_bracket_for(pitch) {
            let down_name = self.name_of(down).unwrap();
            let up_name = self.name_of(up).unwrap();
            let down_modifier = down_name.accidental().sharpen();
            let up_modifier = up_name.accidental().flatten();
            if let Some(down_modifier) = down_modifier {
                if down_modifier.offset_value() < 2 {
                    return (down_name, down, Some(down_modifier));
                }
            }
            (up_name, up, up_modifier)
        } else {
            (self.name_of(pitch).unwrap(), pitch, None)
        }
    }

    /*
    /// Returns 0 for the Middle C/C#/Cb position.
    /// Returns positive numbers for the treble clef.
    /// Returns negative numbers for the bass clef.
    pub fn staff_position(&self, pitch: u8) -> (i16, Option<Accidental>) {
        let (pitch, acc) = if self.contains(pitch) {
            (pitch, None)
        } else {
            let closest = self.closest_scale_match(pitch);
            (closest.0, Some(closest.2))
        };
        let mut steps = self.diatonic_steps_between(self.middle_c(), pitch)
                .unwrap() as i16;
        if pitch < self.middle_c() {
            steps = -steps;
        }
        (steps, acc)
    }
    */
}

struct ScaleUpIterator {
    pattern: ScalePattern,
    current: u8,
    count: usize,
}

impl Iterator for ScaleUpIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= 127 {
            let result = Some(self.current);
            self.current += self.pattern.jump(self.count);
            self.count += 1;
            result
        } else {
            None
        }
    }
}

struct ScaleDownIterator {
    pattern: ScalePattern,
    current: u8,
    count: usize,
}

impl Iterator for ScaleDownIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.pattern.jump(self.count) {
            let result = Some(self.current);
            self.current -= self.pattern.jump(self.count);
            self.count += 1;
            result
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct ScalePattern {
    num_jumps: usize,
    jumps: [u8; 8],
}

impl ScalePattern {
    fn jump(&self, count: usize) -> u8 {
        self.jumps[count % self.num_jumps]
    }

    fn reversed(&self) -> Self {
        let mut result = *self;
        for i in 0..self.num_jumps {
            result.jumps[i] = self.jumps[self.num_jumps - i - 1];
        }
        result
    }

    fn standard(intervals: [u8; 7]) -> Self {
        let mut jumps = [0; 8];
        for (i, j) in intervals.iter().enumerate() {
            jumps[i] = *j;
        }
        Self {
            num_jumps: 7,
            jumps,
        }
    }

    fn mode_rotation(rotation: usize) -> Self {
        let major = [2, 2, 1, 2, 2, 2, 1];
        let mut destination = [0; 7];
        for i in 0..major.len() {
            destination[i] = major[(i + rotation) % major.len()];
        }
        Self::standard(destination)
    }
}
