pub mod analyzer;

use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    fmt::Display,
};

use bare_metal_modulo::{MNum, ModNum};
use enum_iterator::{Sequence, all};
use midi_fundsp::note_velocity_from;
use midi_msg::MidiMsg;
use midi_note_recorder::Recording;

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
    letter: NoteLetter,
    modifier: Accidental,
}

impl NoteName {
    pub fn name_of(pitch: u8) -> Self {
        let (letter, modifier) = MAJOR_ROOT_IDS[(pitch % 12) as usize];
        Self { letter, modifier }
    }

    pub fn reference_pitch(&self) -> Option<u8> {
        self.modifier.pitch_shift(self.letter.natural_pitch())
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

impl ChordName {
    pub fn from_active_pitches(active: ActivePitches) -> Option<Self> {
        SimpleChordInfo::new(active).map(|info| info.mode())
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
        NoteName {
            letter: self.letter,
            modifier: self.modifier,
        }
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
            result[end].modifier = result[end].modifier.flatten().unwrap();
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
            if let Some(name) = ChordName::from_active_pitches(*p) {
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
        ActivePitches, ChordName, MAJOR_ROOT_IDS, MINOR_ROOT_IDS, NoteName, PitchSequence,
    };

    use crate::Accidental::Flat as F;
    use crate::Accidental::Natural as N;
    use crate::Accidental::Sharp as S;
    use crate::ChordMode as CM;
    use crate::NoteLetter as NL;
    use crate::ScaleMode as SM;

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
        for (pitch, letter, modifier) in note_name {
            assert_eq!(NoteName::name_of(pitch), NoteName { letter, modifier });
        }
    }

    #[test]
    fn test_ascending_scale() {
        let scale = SM::Major.rooted(NoteName {
            letter: NL::C,
            modifier: N,
        });
        let c_notes = scale.notes_going_up().collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24]
        );
    }

    #[test]
    fn test_descending_scale() {
        let c_notes = SM::Major
            .rooted(NoteName {
                letter: NL::C,
                modifier: N,
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
            letter: NL::C,
            modifier: N,
        };
        let root2 = NoteName {
            letter: NL::F,
            modifier: S,
        };
        let root3 = NoteName {
            letter: NL::B,
            modifier: F,
        };
        for (root, mode, current, interval, expected) in [
            (root1, SM::Major, 60, 3, 64),
            (root1, SM::Minor, 60, 3, 63),
            (root1, SM::Phrygian, 60, 2, 61),
            (root2, SM::MelodicMinor, 66, 1, 66),
            (root2, SM::MelodicMinor, 66, 6, 75),
            (root3, SM::MelodicMinor, 58, 7, 69),
        ] {
            let scale = mode.rooted(root);
            assert_eq!(scale.note_up(current, interval).unwrap(), expected);
        }
    }

    #[test]
    fn test_note_down() {
        let root1 = NoteName {
            letter: NL::C,
            modifier: N,
        };
        let root2 = NoteName {
            letter: NL::F,
            modifier: S,
        };
        let root3 = NoteName {
            letter: NL::B,
            modifier: F,
        };
        for (root, mode, current, interval, expected) in [
            (root1, SM::Major, 60, 3, 57),
            (root1, SM::Minor, 60, 3, 56),
            (root1, SM::Phrygian, 60, 2, 58),
            (root2, SM::MelodicMinor, 66, 2, 64),
            (root2, SM::MelodicMinor, 66, 3, 62),
            (root3, SM::MelodicMinor, 58, 7, 48),
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
            assert_eq!(expected[i], SM::Major.rooted(note).middle_c());
        }
    }

    #[test]
    fn test_diatonic_intervals() {
        for (root, mode, p1, p2, expected) in [
            (71, SM::Major, 70, 75, Some(3)),
            (71, SM::Major, 75, 70, Some(3)),
            (71, SM::Major, 70, 74, None),
            (67, SM::Major, 71, 71, Some(0)),
            (62, SM::Dorian, 65, 74, Some(5)),
        ] {
            let root = NoteName::name_of(root);
            let scale = mode.rooted(root);
            assert_eq!(scale.diatonic_steps_between(p1, p2), expected);
        }
    }

    #[test]
    fn test_round_up() {
        for (root, mode, pitch, expected) in [(65, SM::Major, 71, 72), (65, SM::Major, 72, 72)] {
            let root = NoteName::name_of(root);
            let scale = mode.rooted(root);
            assert_eq!(scale.round_up(pitch), expected);
        }
    }

    #[test]
    fn test_round_down() {
        for (root, mode, pitch, expected) in [(65, SM::Major, 71, 70), (65, SM::Major, 72, 72)] {
            let root = NoteName::name_of(root);
            let scale = mode.rooted(root);
            assert_eq!(scale.round_down(pitch), expected);
        }
    }

    #[test]
    fn test_note_letters() {
        for (scale, root, letters) in [
            (
                SM::Major,
                60,
                [
                    (0, NL::C),
                    (2, NL::D),
                    (4, NL::E),
                    (5, NL::F),
                    (7, NL::G),
                    (9, NL::A),
                    (11, NL::B),
                    (12, NL::C),
                    (14, NL::D),
                    (16, NL::E),
                    (17, NL::F),
                    (19, NL::G),
                    (21, NL::A),
                    (23, NL::B),
                    (24, NL::C),
                ],
            ),
            (
                SM::Major,
                59,
                [
                    (11, NL::B),
                    (13, NL::C),
                    (15, NL::D),
                    (16, NL::E),
                    (18, NL::F),
                    (20, NL::G),
                    (22, NL::A),
                    (23, NL::B),
                    (25, NL::C),
                    (27, NL::D),
                    (28, NL::E),
                    (30, NL::F),
                    (32, NL::G),
                    (34, NL::A),
                    (35, NL::B),
                ],
            ),
            (
                SM::Minor,
                58,
                [
                    (10, NL::B),
                    (12, NL::C),
                    (13, NL::D),
                    (15, NL::E),
                    (17, NL::F),
                    (18, NL::G),
                    (20, NL::A),
                    (22, NL::B),
                    (24, NL::C),
                    (25, NL::D),
                    (27, NL::E),
                    (29, NL::F),
                    (30, NL::G),
                    (32, NL::A),
                    (34, NL::B),
                ],
            ),
            (
                SM::Minor,
                60,
                [
                    (0, NL::C),
                    (2, NL::D),
                    (3, NL::E),
                    (5, NL::F),
                    (7, NL::G),
                    (8, NL::A),
                    (10, NL::B),
                    (12, NL::C),
                    (14, NL::D),
                    (15, NL::E),
                    (17, NL::F),
                    (19, NL::G),
                    (20, NL::A),
                    (22, NL::B),
                    (24, NL::C),
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
                SM::Major,
                60,
                [
                    (0, NL::C, N),
                    (2, NL::D, N),
                    (4, NL::E, N),
                    (5, NL::F, N),
                    (7, NL::G, N),
                    (9, NL::A, N),
                    (11, NL::B, N),
                    (12, NL::C, N),
                    (14, NL::D, N),
                    (16, NL::E, N),
                    (17, NL::F, N),
                    (19, NL::G, N),
                    (21, NL::A, N),
                    (23, NL::B, N),
                    (24, NL::C, N),
                ],
            ),
            (
                SM::Major,
                59,
                [
                    (11, NL::B, N),
                    (13, NL::C, S),
                    (15, NL::D, S),
                    (16, NL::E, N),
                    (18, NL::F, S),
                    (20, NL::G, S),
                    (22, NL::A, S),
                    (23, NL::B, N),
                    (25, NL::C, S),
                    (27, NL::D, S),
                    (28, NL::E, N),
                    (30, NL::F, S),
                    (32, NL::G, S),
                    (34, NL::A, S),
                    (35, NL::B, N),
                ],
            ),
            (
                SM::Minor,
                58,
                [
                    (10, NL::B, F),
                    (12, NL::C, N),
                    (13, NL::D, F),
                    (15, NL::E, F),
                    (17, NL::F, N),
                    (18, NL::G, F),
                    (20, NL::A, F),
                    (22, NL::B, F),
                    (24, NL::C, N),
                    (25, NL::D, F),
                    (27, NL::E, F),
                    (29, NL::F, N),
                    (30, NL::G, F),
                    (32, NL::A, F),
                    (34, NL::B, F),
                ],
            ),
            (
                SM::Minor,
                60,
                [
                    (0, NL::C, N),
                    (2, NL::D, N),
                    (3, NL::E, F),
                    (5, NL::F, N),
                    (7, NL::G, N),
                    (8, NL::A, F),
                    (10, NL::B, F),
                    (12, NL::C, N),
                    (14, NL::D, N),
                    (15, NL::E, F),
                    (17, NL::F, N),
                    (19, NL::G, N),
                    (20, NL::A, F),
                    (22, NL::B, F),
                    (24, NL::C, N),
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
            (SM::Major, 60, vec![]),
            (SM::Minor, 60, vec![NL::B, NL::A, NL::E]),
            (SM::Major, 62, vec![]),
            (SM::Major, 17, vec![NL::B]),
            (SM::Minor, 17, vec![NL::E, NL::D, NL::B, NL::A]),
            (SM::Major, 61, vec![NL::D, NL::B, NL::A, NL::G, NL::E]),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            assert_eq!(target, rooted.all_flats().collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_all_sharps() {
        for (scale, root, target) in [
            (SM::Major, 60, vec![]),
            (SM::Minor, 60, vec![]),
            (SM::Major, 62, vec![NL::C, NL::F]),
            (SM::Major, 59, vec![NL::A, NL::G, NL::F, NL::D, NL::C]),
            (
                SM::Major,
                18,
                vec![NL::F, NL::E, NL::D, NL::C, NL::A, NL::G],
            ),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            assert_eq!(target, rooted.all_sharps().collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_diatonic_bracket() {
        for (scale, root, note, expected) in [
            (SM::Major, 60, 61, Some((60, 62))),
            (SM::Minor, 69, 61, Some((60, 62))),
            (SM::Major, 67, 73, Some((72, 74))),
            (SM::Major, 67, 72, None),
            (SM::Major, 59, 67, Some((66, 68))),
            (SM::Augmented, 60, 65, Some((64, 67))),
            (SM::Augmented, 60, 66, Some((64, 67))),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            assert_eq!(expected, rooted.diatonic_bracket_for(note));
        }
    }

    #[test]
    fn test_mode_iterator() {
        for (scale, letter, expected) in [
            (
                SM::Major,
                NL::D,
                vec![
                    NL::D,
                    NL::E,
                    NL::F,
                    NL::G,
                    NL::A,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
                ],
            ),
            (
                SM::Minor,
                NL::A,
                vec![
                    NL::A,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
                    NL::F,
                    NL::G,
                    NL::A,
                    NL::B,
                ],
            ),
            (
                SM::Dorian,
                NL::F,
                vec![
                    NL::F,
                    NL::G,
                    NL::A,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
                    NL::F,
                    NL::G,
                ],
            ),
            (
                SM::Augmented,
                NL::C,
                vec![
                    NL::C,
                    NL::D,
                    NL::E,
                    NL::G,
                    NL::G,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
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
            let scale = SM::MelodicMinor.rooted(root);
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
            (SM::Major, 60, 72, NL::C, N, 72, None),
            (SM::Major, 60, 73, NL::C, N, 72, Some(S)),
            (SM::Major, 59, 65, NL::E, N, 64, Some(S)),
            (SM::Major, 59, 67, NL::G, S, 68, Some(N)),
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
            (SM::Major, 60, 72, NL::C, N, 72, None),
            (SM::Major, 60, 73, NL::D, N, 74, Some(F)),
            (SM::Major, 61, 71, NL::C, N, 72, Some(F)),
            (SM::Major, 61, 69, NL::A, F, 68, Some(N)),
        ] {
            let rooted = scale.rooted(NoteName::name_of(root));
            let (name, diatonic_pitch, descend) = rooted.descending_match(pitch);
            assert_eq!(expected_letter, name.letter);
            assert_eq!(expected_modifier, name.modifier);
            assert_eq!(expected_pitch, diatonic_pitch);
            assert_eq!(expected_descend, descend);
        }
    }

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
            let chord_name = ChordName {
                letter,
                modifier,
                mode,
            };
            let expected = notes
                .iter()
                .copied()
                .map(|(letter, modifier)| NoteName { letter, modifier })
                .collect::<Vec<_>>();
            assert_eq!(expected, chord_name.note_names());
        }
    }

    #[test]
    fn test_missing_chord_notes_from() {
        for (chord_root, chord_mode, scale_root, scale_mode, expected) in [
            (60, CM::Major, 60, SM::Major, vec![]),
            (60, CM::Major, 67, SM::Major, vec![]),
            (62, CM::Major, 60, SM::Major, vec![(NL::F, S)]),
            (60, CM::Dominant7, 60, SM::Major, vec![(NL::B, F)]),
            (57, CM::Minor, 59, SM::Major, vec![(NL::A, N), (NL::C, N)]),
        ] {
            let chord_root = NoteName::name_of(chord_root);
            let scale_root = NoteName::name_of(scale_root);
            let chord_name = ChordName {
                letter: chord_root.letter,
                modifier: chord_root.modifier,
                mode: chord_mode,
            };
            let rooted_scale = scale_mode.rooted(scale_root);
            let missing = chord_name.missing_chord_tones_from(&rooted_scale);
            let expected_missing = expected
                .iter()
                .copied()
                .map(|(letter, modifier)| NoteName { letter, modifier })
                .collect::<Vec<_>>();
            assert_eq!(expected_missing, missing);
        }
    }
}
