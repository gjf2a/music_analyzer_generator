pub mod analyzer;
pub mod chords;
pub mod notes;
pub mod scales;

use std::collections::VecDeque;

use midi_fundsp::note_velocity_from;
use midi_msg::MidiMsg;
use midi_note_recorder::Recording;

use crate::{
    chords::{Chord, ChordName},
    notes::{Accidental, NoteLetter, NoteName},
};

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

#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct ActivePitches {
    on: u128,
}

impl ActivePitches {
    pub fn update_from(&mut self, msg: &MidiMsg) {
        if let Some((pitch, velocity)) = note_velocity_from(msg) {
            if velocity > 0 {
                self.set(pitch);
            } else {
                self.clear(pitch);
            }
        }
    }

    pub fn set(&mut self, pitch: u8) {
        self.on |= 1 << pitch;
    }

    pub fn clear(&mut self, pitch: u8) {
        self.on &= !(1 << pitch);
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

    pub fn pitches_activated(&self, old: ActivePitches) -> ActivePitches {
        self.iter().filter(move |p| !old.is_active(*p)).collect()
    }

    pub fn pitches_cleared(&self, old: ActivePitches) -> ActivePitches {
        old.pitches_activated(*self)
    }
}

impl FromIterator<u8> for ActivePitches {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut result = Self::default();
        for pitch in iter {
            result.set(pitch);
        }
        result
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
                pending = Some((Chord::new(name, *p), *t));
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

    use crate::chords::ChordMode as CM;
    use crate::notes::Accidental::Flat as F;
    use crate::notes::Accidental::Natural as N;
    use crate::notes::Accidental::Sharp as S;
    use crate::notes::NoteLetter as NL;
    use crate::scales::ScaleMode as SM;

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
    fn test_ascending_scale() {
        let scale = SM::Major.rooted(NoteName::new(NL::C, N));
        let c_notes = scale.notes_going_up().collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24]
        );
    }

    #[test]
    fn test_descending_scale() {
        let c_notes = SM::Major
            .rooted(NoteName::new(NL::C, N))
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
        let root1 = NoteName::new(NL::C, N);
        let root2 = NoteName::new(NL::F, S);
        let root3 = NoteName::new(NL::B, F);
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
        let root1 = NoteName::new(NL::C, N);
        let root2 = NoteName::new(NL::F, S);
        let root3 = NoteName::new(NL::B, F);
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
            let note = NoteName::new(MAJOR_ROOT_IDS[i].0, MAJOR_ROOT_IDS[i].1);
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
                let name = NoteName::new(letter, modifier);
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
            let root = NoteName::new(letter, modifier);
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
            while ups[0].1.letter() != letter {
                ups.pop_front();
                dns.pop_back();
            }
            while dns[0].1.letter() != letter {
                ups.pop_back();
                dns.pop_front();
            }
            assert_eq!(ups.len(), dns.len());
            for ui in 0..ups.len() {
                let di = dns.len() - ui - 1;
                assert_eq!(ups[ui].1.letter(), dns[di].1.letter());
                if ui % 7 < 5 {
                    assert_eq!(ups[ui], dns[di]);
                } else {
                    assert_eq!(ups[ui].0, dns[di].0 + 1);
                    assert_eq!(
                        ups[ui].1.accidental().offset_value(),
                        dns[di].1.accidental().offset_value() + 1
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
            assert_eq!(expected_letter, name.letter());
            assert_eq!(expected_modifier, name.accidental());
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
            assert_eq!(expected_letter, name.letter());
            assert_eq!(expected_modifier, name.accidental());
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
            let chord_name = ChordName::new(letter, modifier, mode);
            let expected = notes
                .iter()
                .copied()
                .map(|(letter, modifier)| NoteName::new(letter, modifier))
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
            let chord_name =
                ChordName::new(chord_root.letter(), chord_root.accidental(), chord_mode);
            let rooted_scale = scale_mode.rooted(scale_root);
            let missing = chord_name.missing_chord_tones_from(&rooted_scale);
            let expected_missing = expected
                .iter()
                .copied()
                .map(|(letter, modifier)| NoteName::new(letter, modifier))
                .collect::<Vec<_>>();
            assert_eq!(expected_missing, missing);
        }
    }
}
