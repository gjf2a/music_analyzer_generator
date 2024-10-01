pub mod generator;

use std::{collections::VecDeque, fmt::Display};

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
            //Accidental::Natural => '\u{266e}',
            Accidental::Natural => ' ',
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
    fn pattern_up(&self) -> ScalePattern {
        match self {
            ScaleMode::Major => ScalePattern::mode_rotation(0),
            ScaleMode::Dorian => ScalePattern::mode_rotation(1),
            ScaleMode::Phrygian => ScalePattern::mode_rotation(2),
            ScaleMode::Lydian => ScalePattern::mode_rotation(3),
            ScaleMode::Mixolydian => ScalePattern::mode_rotation(4),
            ScaleMode::Minor => ScalePattern::mode_rotation(5),
            ScaleMode::Locrian => ScalePattern::mode_rotation(6),
            ScaleMode::HarmonicMinor => ScalePattern::standard([2, 1, 2, 2, 1, 3, 1]),
            ScaleMode::MelodicMinor => ScalePattern::standard([2, 1, 2, 2, 2, 2, 1]),
            ScaleMode::WholeTone => ScalePattern {
                num_jumps: 6,
                jumps: [2, 2, 2, 2, 2, 2, 0, 0],
            },
            ScaleMode::Diminished => ScalePattern {
                num_jumps: 8,
                jumps: [2, 1, 2, 1, 2, 1, 2, 1],
            },
            ScaleMode::Augmented => ScalePattern {
                num_jumps: 6,
                jumps: [3, 1, 3, 1, 3, 1, 0, 0],
            },
        }
    }

    fn pattern_down(&self) -> ScalePattern {
        match self {
            ScaleMode::MelodicMinor => ScalePattern::mode_rotation(5),
            _ => self.pattern_up(),
        }
        .reversed()
    }

    pub fn notes_going_up(&self, root: NoteName) -> impl Iterator<Item = u8> {
        ScaleUpIterator {
            pattern: self.pattern_up(),
            current: root.lowest_midi_note(),
            count: 0,
        }
    }

    pub fn notes_going_down(&self, root: NoteName) -> impl Iterator<Item = u8> {
        let root_note = root.lowest_midi_note();
        ScaleDownIterator {
            pattern: self.pattern_down(),
            current: root_note + if root_note > 7 { 108 } else { 120 },
            count: 0,
        }
    }

    pub fn note_up(&self, root: NoteName, current: u8, interval: usize) -> Option<u8> {
        self.notes_going_up(root)
            .skip_while(|n| *n < current)
            .skip(interval - 1)
            .next()
    }

    pub fn note_down(&self, root: NoteName, current: u8, interval: usize) -> Option<u8> {
        self.notes_going_down(root)
            .skip_while(|n| *n > current)
            .skip(interval - 1)
            .next()
    }
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
    use std::collections::BTreeSet;

    use midi_msg::Channel;
    use midi_note_recorder::{midi_msg_from, Recording};
    use rand::Rng;

    use crate::{Accidental, ActivePitches, NoteLetter, NoteName, PitchSequence, ScaleMode};

    #[test]
    fn test_ascending_scale() {
        let c_notes = ScaleMode::Major
            .notes_going_up(NoteName {
                letter: NoteLetter::C,
                modifier: Accidental::Natural,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24]
        );
    }

    #[test]
    fn test_descending_scale() {
        let c_notes = ScaleMode::Major
            .notes_going_down(NoteName {
                letter: NoteLetter::C,
                modifier: Accidental::Natural,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![120, 119, 117, 115, 113, 112, 110, 108, 107, 105, 103, 101, 100, 98, 96]
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
            assert_eq!(mode.note_up(root, current, interval).unwrap(), expected);
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
            assert_eq!(mode.note_down(root, current, interval).unwrap(), expected);
        }
    }

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
            assert_eq!(format!("{}", chords[i].1), chord_str);
        }
    }
}
