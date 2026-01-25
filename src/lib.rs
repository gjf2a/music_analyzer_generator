pub mod generator;

use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Display,
};

use bare_metal_modulo::{MNum, ModNum};
use enum_iterator::{Sequence, all};
use midi_fundsp::note_velocity_from;
use midi_msg::MidiMsg;
use midi_note_recorder::Recording;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NoteName {
    letter: NoteLetter,
    modifier: Accidental,
}

impl NoteName {
    pub fn name_of(pitch: u8) -> Self {
        let (letter, modifier) = MAJOR_ROOT_IDS[(pitch % 12) as usize];
        Self { letter, modifier }
    }

    pub fn full_name_for(letter: NoteLetter, pitch: u8) -> Self {
        let offset = letter.steps_above_natural(pitch);
        Self {
            letter,
            modifier: match offset {
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
        self.modifier
            .pitch_shift(self.letter.natural_pitch())
            .unwrap()
    }
}

impl Display for NoteName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}{}", self.letter, self.modifier.symbol(),)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Chord {
    name: ChordName,
    notes: ActivePitches,
}

impl Chord {
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

    pub fn compact_name(&self) -> String {
        let base_note_letter = format!("{:?}{}", self.note, self.accidental.symbol());
        let note_letter = base_note_letter.trim();
        match self.mode {
            ChordMode::Major => note_letter.to_owned(),
            ChordMode::Minor => format!("{note_letter}m"),
            ChordMode::Diminished => format!("{note_letter}\u{00b0}"),
            ChordMode::Augmented => format!("{note_letter}+"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
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

    fn letter_iterator(&self, root_letter: NoteLetter) -> ScaleLetterIterator {
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

struct ScaleLetterIterator {
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

pub struct RootedScale {
    mode: ScaleMode,
    root: NoteName,
    notes2letters_up: BTreeMap<u8, NoteLetter>,
    notes2names_up: BTreeMap<u8, NoteName>,
    notes2letters_down: BTreeMap<u8, NoteLetter>,
    notes2names_down: BTreeMap<u8, NoteName>,
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
        };
        let notes2letters_up = result.all_diatonic_note_letters_up().collect();
        result.notes2letters_up = notes2letters_up;
        let notes2names = result.all_diatonic_notes_up().collect();
        result.notes2names_up = notes2names;
        let notes2letters_down = result.all_diatonic_note_letters_down().collect();
        result.notes2letters_down = notes2letters_down;
        let notes2names_down = result.all_diatonic_notes_down().collect();
        result.notes2names_down = notes2names_down;
        result
    }

    pub fn all_diatonic_note_letters_up(&self) -> impl Iterator<Item = (u8, NoteLetter)> {
        self.notes_going_up()
            .zip(self.mode.letter_iterator(self.root.letter))
    }

    pub fn all_diatonic_notes_up(&self) -> impl Iterator<Item = (u8, NoteName)> {
        self.all_diatonic_note_letters_up()
            .map(|(pitch, letter)| (pitch, NoteName::full_name_for(letter, pitch)))
    }

    pub fn all_diatonic_note_letters_down(&self) -> impl Iterator<Item = (u8, NoteLetter)> {
        self.notes_going_down()
            .zip(self.mode.letter_iterator(self.root.letter).rev())
    }

    pub fn all_diatonic_notes_down(&self) -> impl Iterator<Item = (u8, NoteName)> {
        self.all_diatonic_note_letters_down()
            .map(|(pitch, letter)| (pitch, NoteName::full_name_for(letter, pitch)))
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
            .filter(|(_, n)| n.modifier == Accidental::Sharp)
            .map(|(_, n)| n.letter)
    }

    pub fn all_flats(&self) -> impl Iterator<Item = NoteLetter> {
        self.all_diatonic_notes_down()
            .take(7)
            .filter(|(_, n)| n.modifier == Accidental::Flat)
            .map(|(_, n)| n.letter)
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
            let down_modifier = down_name.modifier.sharpen();
            let up_modifier = up_name.modifier.flatten();
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
            let down_modifier = down_name.modifier.sharpen();
            let up_modifier = up_name.modifier.flatten();
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

impl ChordMode {
    pub fn scales(&self) -> Vec<ScaleMode> {
        match self {
            ChordMode::Major => vec![ScaleMode::Major, ScaleMode::Lydian, ScaleMode::Mixolydian],
            ChordMode::Minor => vec![
                ScaleMode::Minor,
                ScaleMode::MelodicMinor,
                ScaleMode::HarmonicMinor,
                ScaleMode::Dorian,
                ScaleMode::Phrygian,
            ],
            ChordMode::Diminished => vec![ScaleMode::Diminished],
            ChordMode::Augmented => vec![ScaleMode::Augmented, ScaleMode::WholeTone],
        }
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct ActivePitches {
    on: u128,
}

impl ActivePitches {
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

#[derive(Clone, Default)]
pub struct PitchSequence {
    seq: Vec<(f64, MidiMsg, ActivePitches)>,
}

impl PitchSequence {
    pub fn new(recording: &Recording) -> Self {
        let mut current = ActivePitches::default();
        let mut result = Self::default();
        let mut queue = recording.midi_queue();
        while let Some((time, msg)) = queue.pop_front() {
            result.push(time, &msg, &mut current);
        }
        result
    }

    fn push(&mut self, time: f64, msg: &MidiMsg, current: &mut ActivePitches) {
        current.update_from(&msg);
        self.seq.push((time, msg.clone(), *current));
    }

    pub fn recording(&self) -> Recording {
        let mut result = Recording::default();
        for (time, msg, _) in self.seq.iter() {
            result.add_message(*time, msg);
        }
        result
    }

    pub fn without_notes_below(&self, min_duration: f64, min_velocity: u8) -> Self {
        let mut result = Self::default();
        let mut current = ActivePitches::default();
        for (i, (t, msg, _)) in self.seq.iter().enumerate() {
            if self.keep_note_without_below(min_duration, min_velocity, i, current) {
                result.push(*t, msg, &mut current);
            }
        }
        result
    }

    fn keep_note_without_below(
        &self,
        min_duration: f64,
        min_velocity: u8,
        i: usize,
        current: ActivePitches,
    ) -> bool {
        if let Some((n, v)) = note_velocity_from(&self.seq[i].1) {
            if v >= min_velocity {
                self.next_off_note_index(i)
                    .map_or(true, |j| (self.seq[j].0 - self.seq[i].0) >= min_duration)
            } else {
                current.is_active(n)
            }
        } else {
            true
        }
    }

    fn next_off_note_index(&self, i: usize) -> Option<usize> {
        if let Some((n, _)) = note_velocity_from(&self.seq[i].1) {
            for j in (i + 1)..self.seq.len() {
                if let Some((nj, vj)) = note_velocity_from(&self.seq[j].1) {
                    if n == nj {
                        return if vj == 0 { Some(j) } else { None };
                    }
                }
            }
        }
        None
    }

    pub fn chords_starts_durations(&self) -> Vec<(Chord, f64, f64)> {
        let mut pending = None;
        let mut result = vec![];
        let mut last_time = 0.0;
        for (t, _, p) in self.seq.iter() {
            if let Some(name) = ChordName::new(*p) {
                if let Some((chord, time)) = pending {
                    result.push((chord, time, *t - time));
                    last_time = time;
                }
                pending = Some((Chord { name, notes: *p }, *t));
            }
        }
        if let Some((chord, time)) = pending {
            result.push((chord, time, time - last_time));
        }
        result
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

pub fn durations_notes_from(recording: &Recording) -> Vec<(f64, u8, u8)> {
    let mut result = Vec::new();
    let mut queue = recording.midi_queue();
    if let Some((mut last_time, mut last_n, mut last_v)) = find_first_note(&mut queue) {
        while let Some((time, msg)) = queue.pop_front() {
            if let Some((n, v)) = note_velocity_from(&msg) {
                if last_v > 0 {
                    if let Some((_, end_n, end_v)) = result.last().copied() {
                        if end_v > 0 {
                            result.push((0.0, end_n, 0));
                        }
                    }
                }

                if v > 0 || n == last_n {
                    result.push((time - last_time, last_n, last_v));
                    last_time = time;
                    last_n = n;
                    last_v = v;
                }
            }
        }
    }
    if result.len() % 2 == 1 {
        let (_, n, _) = result.last().unwrap();
        result.push((0.0, *n, 0));
    }
    result
}

fn find_first_note(queue: &mut VecDeque<(f64, MidiMsg)>) -> Option<(f64, u8, u8)> {
    while let Some((time, msg)) = queue.pop_front() {
        if let Some((n, v)) = note_velocity_from(&msg) {
            return Some((time, n, v));
        }
    }
    None
}

pub fn partitioned_melody(melody: &Vec<(f64, u8, u8)>, stop_length: usize) -> Vec<ClosedInterval> {
    pm_help(ClosedInterval::indices(melody), melody, stop_length)
}

fn pm_help(
    interval: ClosedInterval,
    melody: &Vec<(f64, u8, u8)>,
    stop_length: usize,
) -> Vec<ClosedInterval> {
    if interval.len() <= stop_length {
        return vec![interval];
    } else {
        let max_time_index = interval
            .iter()
            .map(|i| (i, melody[i].0))
            .max_by(|(_, t1), (_, t2)| t1.partial_cmp(t2).unwrap())
            .unwrap()
            .0;
        if max_time_index < interval.end {
            let (i1, i2) = interval.divided(max_time_index);
            let mut v1 = pm_help(i1, melody, stop_length);
            let mut v2 = pm_help(i2, melody, stop_length);
            v1.append(&mut v2);
            v1
        } else {
            let sub = ClosedInterval {
                start: interval.start,
                end: interval.end - 1,
            };
            let mut v = pm_help(sub, melody, stop_length);
            v.last_mut().unwrap().end += 1;
            v
        }
    }
}

pub fn duration_clusters(melody: &Vec<(f64, u8, u8)>, stop_length: usize) -> Vec<Vec<f64>> {
    let partitioned = partitioned_melody(melody, stop_length);
    let mut result = vec![];
    for interval in partitioned.iter() {
        result.push(interval.iter().map(|i| melody[i].0).collect());
    }
    result
}

pub fn consolidated_note_rest_times(durations_notes: &Vec<(f64, u8, u8)>) -> Vec<(f64, u8, u8)> {
    (0..durations_notes.len())
        .step_by(2)
        .map(|i| {
            (
                durations_notes[i].0 + durations_notes[i + 1].0,
                durations_notes[i].1,
                durations_notes[i].2,
            )
        })
        .collect()
}

#[derive(Copy, Clone, Debug, Default)]
pub struct ClosedInterval {
    start: usize,
    end: usize,
}

impl ClosedInterval {
    pub fn indices<T>(v: &Vec<T>) -> Self {
        Self {
            start: 0,
            end: v.len() - 1,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> {
        self.start..=self.end
    }

    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start > self.end
    }

    pub fn len(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            self.end - self.start + 1
        }
    }

    pub fn contains(&self, i: usize) -> bool {
        self.start <= i && i <= self.end
    }

    pub fn divided(&self, division_end: usize) -> (Self, Self) {
        assert!(self.start <= division_end && division_end < self.end);
        (
            Self {
                start: self.start,
                end: division_end,
            },
            Self {
                start: division_end + 1,
                end: self.end,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, VecDeque};

    use midi_msg::Channel;
    use midi_note_recorder::{Recording, midi_msg_from};
    use rand::Rng;

    use crate::{
        Accidental, ActivePitches, MAJOR_ROOT_IDS, MINOR_ROOT_IDS, NoteLetter, NoteName,
        PitchSequence, ScaleMode,
    };

    #[test]
    fn test_note_names() {
        let note_name = [
            (60, NoteLetter::C, Accidental::Natural),
            (61, NoteLetter::D, Accidental::Flat),
            (62, NoteLetter::D, Accidental::Natural),
            (63, NoteLetter::E, Accidental::Flat),
            (64, NoteLetter::E, Accidental::Natural),
            (65, NoteLetter::F, Accidental::Natural),
            (66, NoteLetter::F, Accidental::Sharp),
            (67, NoteLetter::G, Accidental::Natural),
            (68, NoteLetter::A, Accidental::Flat),
            (69, NoteLetter::A, Accidental::Natural),
            (70, NoteLetter::B, Accidental::Flat),
            (71, NoteLetter::B, Accidental::Natural),
        ];
        for (pitch, letter, modifier) in note_name {
            assert_eq!(NoteName::name_of(pitch), NoteName { letter, modifier });
        }
    }

    #[test]
    fn test_ascending_scale() {
        let scale = ScaleMode::Major.rooted(NoteName {
            letter: NoteLetter::C,
            modifier: Accidental::Natural,
        });
        let c_notes = scale.notes_going_up().collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24]
        );
    }

    #[test]
    fn test_descending_scale() {
        let c_notes = ScaleMode::Major
            .rooted(NoteName {
                letter: NoteLetter::C,
                modifier: Accidental::Natural,
            })
            .notes_going_down()
            .collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![
                120, 119, 117, 115, 113, 112, 110, 108, 107, 105, 103, 101, 100, 98, 96
            ]
        );
    }

    #[test]
    fn test_note_up() {
        let root1 = NoteName {
            letter: NoteLetter::C,
            modifier: Accidental::Natural,
        };
        let root2 = NoteName {
            letter: NoteLetter::F,
            modifier: Accidental::Sharp,
        };
        let root3 = NoteName {
            letter: NoteLetter::B,
            modifier: Accidental::Flat,
        };
        for (root, mode, current, interval, expected) in [
            (root1, ScaleMode::Major, 60, 3, 64),
            (root1, ScaleMode::Minor, 60, 3, 63),
            (root1, ScaleMode::Phrygian, 60, 2, 61),
            (root2, ScaleMode::MelodicMinor, 66, 1, 66),
            (root2, ScaleMode::MelodicMinor, 66, 6, 75),
            (root3, ScaleMode::MelodicMinor, 58, 7, 69),
        ] {
            let scale = mode.rooted(root);
            assert_eq!(scale.note_up(current, interval).unwrap(), expected);
        }
    }

    #[test]
    fn test_note_down() {
        let root1 = NoteName {
            letter: NoteLetter::C,
            modifier: Accidental::Natural,
        };
        let root2 = NoteName {
            letter: NoteLetter::F,
            modifier: Accidental::Sharp,
        };
        let root3 = NoteName {
            letter: NoteLetter::B,
            modifier: Accidental::Flat,
        };
        for (root, mode, current, interval, expected) in [
            (root1, ScaleMode::Major, 60, 3, 57),
            (root1, ScaleMode::Minor, 60, 3, 56),
            (root1, ScaleMode::Phrygian, 60, 2, 58),
            (root2, ScaleMode::MelodicMinor, 66, 2, 64),
            (root2, ScaleMode::MelodicMinor, 66, 3, 62),
            (root3, ScaleMode::MelodicMinor, 58, 7, 48),
        ] {
            let scale = mode.rooted(root);
            assert_eq!(scale.note_down(current, interval).unwrap(), expected);
        }
    }

    #[test]
    fn test_active_pitches() {
        let mut rng = rand::rng();
        let mut active = ActivePitches::default();
        let mut active_tester = BTreeSet::new();
        for _ in 0..100 {
            if active.len() == 0 || rng.random_bool(0.5) {
                let note = rng.random_range(0..=127);
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
                let remove = pitches[rng.random_range(0..pitches.len())];
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
        let expected = "A  Major ([61, 64, 69])
A  Major ([61, 64, 69])
B  Major ([59, 63, 66])
E♭ Minor ([58, 59, 63, 66])
B  Major ([59, 63, 66])
B  Major ([59, 63, 66])
B  Major ([59, 63, 66])
E  Major ([59, 64, 68])
E  Major ([59, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
A  Major ([61, 64, 69])
A  Major ([61, 64, 69])
B  Major ([59, 63, 66])
B  Major ([59, 63, 66])
E  Major ([59, 64, 68])
E  Major ([59, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
A  Major ([61, 64, 69])
A  Major ([61, 64, 69])
B  Major ([59, 63, 66])
B  Major ([59, 63, 66])
B  Major ([59, 63, 66])
E  Major ([59, 64, 68])
E  Major ([59, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
C♯ Minor ([61, 64, 68])
A  Major ([57, 61, 64])
B  Major ([59, 63, 66])
E♭ Diminished ([57, 59, 63, 66])
B  Major ([59, 63, 66])
B  Major ([59, 63, 66])";
        let chords = PitchSequence::new(&recording).chords_starts_durations();
        for (i, chord_str) in expected.lines().enumerate() {
            assert_eq!(format!("{}", chords[i].0), chord_str);
        }
    }

    #[test]
    fn test_chord_compact() {
        let recording = Recording::from_file("healing4").unwrap();
        let expected = [
            "A", "A", "B", "E♭m", "B", "B", "B", "E", "E", "C♯m", "C♯m", "C♯m", "A", "A", "B", "B",
            "E", "E", "C♯m", "C♯m", "C♯m", "A", "A", "B", "B", "B", "E", "E", "C♯m", "C♯m", "C♯m",
            "A", "B", "E♭°", "B", "B",
        ];
        let chords = PitchSequence::new(&recording).chords_starts_durations();
        for (i, chord_str) in expected.iter().enumerate() {
            let c = chords[i].0.name().compact_name();
            assert_eq!(c, *chord_str);
        }
    }

    #[test]
    fn test_middle_c() {
        let expected = [60, 60, 61, 60, 61, 60, 61, 60, 60, 61, 60, 61];
        for i in 0..expected.len() {
            let note = NoteName {
                letter: MAJOR_ROOT_IDS[i].0,
                modifier: MAJOR_ROOT_IDS[i].1,
            };
            assert_eq!(expected[i], ScaleMode::Major.rooted(note).middle_c());
        }
    }

    #[test]
    fn test_diatonic_intervals() {
        for (root, mode, p1, p2, expected) in [
            (71, ScaleMode::Major, 70, 75, Some(3)),
            (71, ScaleMode::Major, 75, 70, Some(3)),
            (71, ScaleMode::Major, 70, 74, None),
            (67, ScaleMode::Major, 71, 71, Some(0)),
            (62, ScaleMode::Dorian, 65, 74, Some(5)),
        ] {
            let root = NoteName::name_of(root);
            let scale = mode.rooted(root);
            assert_eq!(scale.diatonic_steps_between(p1, p2), expected);
        }
    }

    #[test]
    fn test_round_up() {
        for (root, mode, pitch, expected) in [
            (65, ScaleMode::Major, 71, 72),
            (65, ScaleMode::Major, 72, 72),
        ] {
            let root = NoteName::name_of(root);
            let scale = mode.rooted(root);
            assert_eq!(scale.round_up(pitch), expected);
        }
    }

    #[test]
    fn test_round_down() {
        for (root, mode, pitch, expected) in [
            (65, ScaleMode::Major, 71, 70),
            (65, ScaleMode::Major, 72, 72),
        ] {
            let root = NoteName::name_of(root);
            let scale = mode.rooted(root);
            assert_eq!(scale.round_down(pitch), expected);
        }
    }

    #[test]
    fn test_note_letters() {
        for (scale, root, letters) in [
            (
                ScaleMode::Major,
                60,
                [
                    (0, NoteLetter::C),
                    (2, NoteLetter::D),
                    (4, NoteLetter::E),
                    (5, NoteLetter::F),
                    (7, NoteLetter::G),
                    (9, NoteLetter::A),
                    (11, NoteLetter::B),
                    (12, NoteLetter::C),
                    (14, NoteLetter::D),
                    (16, NoteLetter::E),
                    (17, NoteLetter::F),
                    (19, NoteLetter::G),
                    (21, NoteLetter::A),
                    (23, NoteLetter::B),
                    (24, NoteLetter::C),
                ],
            ),
            (
                ScaleMode::Major,
                59,
                [
                    (11, NoteLetter::B),
                    (13, NoteLetter::C),
                    (15, NoteLetter::D),
                    (16, NoteLetter::E),
                    (18, NoteLetter::F),
                    (20, NoteLetter::G),
                    (22, NoteLetter::A),
                    (23, NoteLetter::B),
                    (25, NoteLetter::C),
                    (27, NoteLetter::D),
                    (28, NoteLetter::E),
                    (30, NoteLetter::F),
                    (32, NoteLetter::G),
                    (34, NoteLetter::A),
                    (35, NoteLetter::B),
                ],
            ),
            (
                ScaleMode::Minor,
                58,
                [
                    (10, NoteLetter::B),
                    (12, NoteLetter::C),
                    (13, NoteLetter::D),
                    (15, NoteLetter::E),
                    (17, NoteLetter::F),
                    (18, NoteLetter::G),
                    (20, NoteLetter::A),
                    (22, NoteLetter::B),
                    (24, NoteLetter::C),
                    (25, NoteLetter::D),
                    (27, NoteLetter::E),
                    (29, NoteLetter::F),
                    (30, NoteLetter::G),
                    (32, NoteLetter::A),
                    (34, NoteLetter::B),
                ],
            ),
            (
                ScaleMode::Minor,
                60,
                [
                    (0, NoteLetter::C),
                    (2, NoteLetter::D),
                    (3, NoteLetter::E),
                    (5, NoteLetter::F),
                    (7, NoteLetter::G),
                    (8, NoteLetter::A),
                    (10, NoteLetter::B),
                    (12, NoteLetter::C),
                    (14, NoteLetter::D),
                    (15, NoteLetter::E),
                    (17, NoteLetter::F),
                    (19, NoteLetter::G),
                    (20, NoteLetter::A),
                    (22, NoteLetter::B),
                    (24, NoteLetter::C),
                ],
            ),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            let values = rooted
                .all_diatonic_note_letters_up()
                .take(letters.len())
                .collect::<Vec<_>>();
            for i in 0..letters.len() {
                assert_eq!(values[i], letters[i]);
            }
        }
    }

    #[test]
    fn test_note_name_letters() {
        for (scale, root, letters) in [
            (
                ScaleMode::Major,
                60,
                [
                    (0, NoteLetter::C, Accidental::Natural),
                    (2, NoteLetter::D, Accidental::Natural),
                    (4, NoteLetter::E, Accidental::Natural),
                    (5, NoteLetter::F, Accidental::Natural),
                    (7, NoteLetter::G, Accidental::Natural),
                    (9, NoteLetter::A, Accidental::Natural),
                    (11, NoteLetter::B, Accidental::Natural),
                    (12, NoteLetter::C, Accidental::Natural),
                    (14, NoteLetter::D, Accidental::Natural),
                    (16, NoteLetter::E, Accidental::Natural),
                    (17, NoteLetter::F, Accidental::Natural),
                    (19, NoteLetter::G, Accidental::Natural),
                    (21, NoteLetter::A, Accidental::Natural),
                    (23, NoteLetter::B, Accidental::Natural),
                    (24, NoteLetter::C, Accidental::Natural),
                ],
            ),
            (
                ScaleMode::Major,
                59,
                [
                    (11, NoteLetter::B, Accidental::Natural),
                    (13, NoteLetter::C, Accidental::Sharp),
                    (15, NoteLetter::D, Accidental::Sharp),
                    (16, NoteLetter::E, Accidental::Natural),
                    (18, NoteLetter::F, Accidental::Sharp),
                    (20, NoteLetter::G, Accidental::Sharp),
                    (22, NoteLetter::A, Accidental::Sharp),
                    (23, NoteLetter::B, Accidental::Natural),
                    (25, NoteLetter::C, Accidental::Sharp),
                    (27, NoteLetter::D, Accidental::Sharp),
                    (28, NoteLetter::E, Accidental::Natural),
                    (30, NoteLetter::F, Accidental::Sharp),
                    (32, NoteLetter::G, Accidental::Sharp),
                    (34, NoteLetter::A, Accidental::Sharp),
                    (35, NoteLetter::B, Accidental::Natural),
                ],
            ),
            (
                ScaleMode::Minor,
                58,
                [
                    (10, NoteLetter::B, Accidental::Flat),
                    (12, NoteLetter::C, Accidental::Natural),
                    (13, NoteLetter::D, Accidental::Flat),
                    (15, NoteLetter::E, Accidental::Flat),
                    (17, NoteLetter::F, Accidental::Natural),
                    (18, NoteLetter::G, Accidental::Flat),
                    (20, NoteLetter::A, Accidental::Flat),
                    (22, NoteLetter::B, Accidental::Flat),
                    (24, NoteLetter::C, Accidental::Natural),
                    (25, NoteLetter::D, Accidental::Flat),
                    (27, NoteLetter::E, Accidental::Flat),
                    (29, NoteLetter::F, Accidental::Natural),
                    (30, NoteLetter::G, Accidental::Flat),
                    (32, NoteLetter::A, Accidental::Flat),
                    (34, NoteLetter::B, Accidental::Flat),
                ],
            ),
            (
                ScaleMode::Minor,
                60,
                [
                    (0, NoteLetter::C, Accidental::Natural),
                    (2, NoteLetter::D, Accidental::Natural),
                    (3, NoteLetter::E, Accidental::Flat),
                    (5, NoteLetter::F, Accidental::Natural),
                    (7, NoteLetter::G, Accidental::Natural),
                    (8, NoteLetter::A, Accidental::Flat),
                    (10, NoteLetter::B, Accidental::Flat),
                    (12, NoteLetter::C, Accidental::Natural),
                    (14, NoteLetter::D, Accidental::Natural),
                    (15, NoteLetter::E, Accidental::Flat),
                    (17, NoteLetter::F, Accidental::Natural),
                    (19, NoteLetter::G, Accidental::Natural),
                    (20, NoteLetter::A, Accidental::Flat),
                    (22, NoteLetter::B, Accidental::Flat),
                    (24, NoteLetter::C, Accidental::Natural),
                ],
            ),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            let values = rooted
                .all_diatonic_notes_up()
                .take(letters.len())
                .collect::<Vec<_>>();
            for i in 0..letters.len() {
                let (pitch, letter, modifier) = letters[i];
                let name = NoteName { letter, modifier };
                assert_eq!(values[i], (pitch, name));
            }
        }
    }

    #[test]
    fn test_all_flats() {
        for (scale, root, target) in [
            (ScaleMode::Major, 60, vec![]),
            (
                ScaleMode::Minor,
                60,
                vec![NoteLetter::B, NoteLetter::A, NoteLetter::E],
            ),
            (ScaleMode::Major, 62, vec![]),
            (ScaleMode::Major, 17, vec![NoteLetter::B]),
            (
                ScaleMode::Minor,
                17,
                vec![NoteLetter::E, NoteLetter::D, NoteLetter::B, NoteLetter::A],
            ),
            (
                ScaleMode::Major,
                61,
                vec![
                    NoteLetter::D,
                    NoteLetter::B,
                    NoteLetter::A,
                    NoteLetter::G,
                    NoteLetter::E,
                ],
            ),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            assert_eq!(target, rooted.all_flats().collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_all_sharps() {
        for (scale, root, target) in [
            (ScaleMode::Major, 60, vec![]),
            (ScaleMode::Minor, 60, vec![]),
            (ScaleMode::Major, 62, vec![NoteLetter::C, NoteLetter::F]),
            (
                ScaleMode::Major,
                59,
                vec![
                    NoteLetter::A,
                    NoteLetter::G,
                    NoteLetter::F,
                    NoteLetter::D,
                    NoteLetter::C,
                ],
            ),
            (
                ScaleMode::Major,
                18,
                vec![
                    NoteLetter::F,
                    NoteLetter::E,
                    NoteLetter::D,
                    NoteLetter::C,
                    NoteLetter::A,
                    NoteLetter::G,
                ],
            ),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            assert_eq!(target, rooted.all_sharps().collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_diatonic_bracket() {
        for (scale, root, note, expected) in [
            (ScaleMode::Major, 60, 61, Some((60, 62))),
            (ScaleMode::Minor, 69, 61, Some((60, 62))),
            (ScaleMode::Major, 67, 73, Some((72, 74))),
            (ScaleMode::Major, 67, 72, None),
            (ScaleMode::Major, 59, 67, Some((66, 68))),
            (ScaleMode::Augmented, 60, 65, Some((64, 67))),
            (ScaleMode::Augmented, 60, 66, Some((64, 67))),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            assert_eq!(expected, rooted.diatonic_bracket_for(note));
        }
    }

    #[test]
    fn test_mode_iterator() {
        for (scale, letter, expected) in [
            (
                ScaleMode::Major,
                NoteLetter::D,
                vec![
                    NoteLetter::D,
                    NoteLetter::E,
                    NoteLetter::F,
                    NoteLetter::G,
                    NoteLetter::A,
                    NoteLetter::B,
                    NoteLetter::C,
                    NoteLetter::D,
                    NoteLetter::E,
                ],
            ),
            (
                ScaleMode::Minor,
                NoteLetter::A,
                vec![
                    NoteLetter::A,
                    NoteLetter::B,
                    NoteLetter::C,
                    NoteLetter::D,
                    NoteLetter::E,
                    NoteLetter::F,
                    NoteLetter::G,
                    NoteLetter::A,
                    NoteLetter::B,
                ],
            ),
            (
                ScaleMode::Dorian,
                NoteLetter::F,
                vec![
                    NoteLetter::F,
                    NoteLetter::G,
                    NoteLetter::A,
                    NoteLetter::B,
                    NoteLetter::C,
                    NoteLetter::D,
                    NoteLetter::E,
                    NoteLetter::F,
                    NoteLetter::G,
                ],
            ),
            (
                ScaleMode::Augmented,
                NoteLetter::C,
                vec![
                    NoteLetter::C,
                    NoteLetter::D,
                    NoteLetter::E,
                    NoteLetter::G,
                    NoteLetter::G,
                    NoteLetter::B,
                    NoteLetter::C,
                    NoteLetter::D,
                    NoteLetter::E,
                ],
            ),
        ] {
            let letters = scale.letter_iterator(letter).take(9).collect::<Vec<_>>();
            assert_eq!(expected, letters);
        }
    }

    #[test]
    fn test_melodic_minor() {
        for (letter, modifier) in MINOR_ROOT_IDS.iter().copied() {
            println!("New loop: {letter:?} {modifier:?}");
            let root = NoteName { letter, modifier };
            let scale = ScaleMode::MelodicMinor.rooted(root);
            let mut ups = scale.all_diatonic_notes_up().collect::<VecDeque<_>>();
            let mut dns = scale.all_diatonic_notes_down().collect::<VecDeque<_>>();
            while ups[ups.len() - 1] != dns[0] {
                ups.pop_back();
            }
            while ups[0] != dns[dns.len() - 1] && ups[0].0 < dns[dns.len() - 1].0 {
                println!("{:?} {:?} {}", ups[0], dns[dns.len() - 1], ups.len());
                ups.pop_front();
            }
            while ups[0] != dns[dns.len() - 1] && ups[0].0 > dns[dns.len() - 1].0 {
                dns.pop_back();
            }
            while ups[0].1.letter != letter {
                ups.pop_front();
                dns.pop_back();
            }
            while dns[0].1.letter != letter {
                ups.pop_back();
                dns.pop_front();
            }
            assert_eq!(ups.len(), dns.len());
            for ui in 0..ups.len() {
                let di = dns.len() - ui - 1;
                assert_eq!(ups[ui].1.letter, dns[di].1.letter);
                if ui % 7 < 5 {
                    assert_eq!(ups[ui], dns[di]);
                } else {
                    assert_eq!(ups[ui].0, dns[di].0 + 1);
                    assert_eq!(
                        ups[ui].1.modifier.offset_value(),
                        dns[di].1.modifier.offset_value() + 1
                    );
                }
            }
        }
    }

    #[test]
    fn test_ascending_match() {
        for (
            scale,
            root,
            pitch,
            expected_letter,
            expected_modifier,
            expected_pitch,
            expected_ascend,
        ) in [
            (
                ScaleMode::Major,
                60,
                72,
                NoteLetter::C,
                Accidental::Natural,
                72,
                None,
            ),
            (
                ScaleMode::Major,
                60,
                73,
                NoteLetter::C,
                Accidental::Natural,
                72,
                Some(Accidental::Sharp),
            ),
            (
                ScaleMode::Major,
                59,
                65,
                NoteLetter::E,
                Accidental::Natural,
                64,
                Some(Accidental::Sharp),
            ),
            (
                ScaleMode::Major,
                59,
                67,
                NoteLetter::G,
                Accidental::Sharp,
                68,
                Some(Accidental::Natural),
            ),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            let (name, diatonic_pitch, ascend) = rooted.ascending_match(pitch);
            assert_eq!(expected_letter, name.letter);
            assert_eq!(expected_modifier, name.modifier);
            assert_eq!(expected_pitch, diatonic_pitch);
            assert_eq!(expected_ascend, ascend);
        }
    }

    #[test]
    fn test_descending_match() {
        for (
            scale,
            root,
            pitch,
            expected_letter,
            expected_modifier,
            expected_pitch,
            expected_descend,
        ) in [
            (
                ScaleMode::Major,
                60,
                72,
                NoteLetter::C,
                Accidental::Natural,
                72,
                None,
            ),
            (
                ScaleMode::Major,
                60,
                73,
                NoteLetter::D,
                Accidental::Natural,
                74,
                Some(Accidental::Flat),
            ),
            (
                ScaleMode::Major,
                61,
                71,
                NoteLetter::C,
                Accidental::Natural,
                72,
                Some(Accidental::Flat),
            ),
            (
                ScaleMode::Major,
                61,
                69,
                NoteLetter::A,
                Accidental::Flat,
                68,
                Some(Accidental::Natural),
            ),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            let (name, diatonic_pitch, descend) = rooted.descending_match(pitch);
            assert_eq!(expected_letter, name.letter);
            assert_eq!(expected_modifier, name.modifier);
            assert_eq!(expected_pitch, diatonic_pitch);
            assert_eq!(expected_descend, descend);
        }
    }
}
