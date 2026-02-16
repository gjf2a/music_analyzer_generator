use std::{
    cmp::{Ordering, min},
    collections::HashMap,
    ops::Index,
};

use hash_histogram::HashHistogram;
use midi_fundsp::note_velocity_from;
use midi_msg::MidiMsg;

use crate::{
    ChordName, NoteDuration, NoteName, PitchSequence,
    notes::Note,
    scales::{RootedScale, all_rooted_scales},
};
use midi_note_recorder::{Recording, Timestamp, TotalDuration};

pub const PHRASE_ENDING_DURATION_MULTIPLIER: NoteDuration = 1.5;
pub const DURATION_BUFFER: NoteDuration = 0.5;
pub const DURATION_MOVING_WINDOW_SIZE: usize = 4;

#[derive(Debug)]
pub struct ChordProgression {
    chords_starts: Vec<(ChordName, Timestamp)>,
    duration: TotalDuration,
}

impl ChordProgression {
    pub fn chord_iter(&self) -> impl Iterator<Item = ChordName> {
        self.chords_starts.iter().map(|(c, _)| *c)
    }

    pub fn chord_start_end_iter(&self) -> impl Iterator<Item = (ChordName, Timestamp, Timestamp)> {
        (0..self.len()).map(|i| self.chord_start_end(i))
    }

    pub fn chord_at_time(&self, timestamp: Timestamp) -> Option<ChordName> {
        self.chord_start_end_iter()
            .find(|(_, start, end)| *start <= timestamp && timestamp <= *end)
            .map(|(chord, _, _)| chord)
    }

    pub fn chord_start_end(&self, index: usize) -> (ChordName, Timestamp, Timestamp) {
        let (chord, start) = self.chords_starts[index];
        let end = if index + 1 == self.len() {
            self.duration
        } else {
            self.chords_starts[index + 1].1
        };
        (chord, start, end)
    }

    pub fn len(&self) -> usize {
        self.chords_starts.len()
    }

    pub fn has_root_chord(&self, scale: &RootedScale) -> bool {
        self.chord_iter().any(|chord| {
            scale.root_name() == chord.root_name()
                && chord.missing_chord_tones_from(scale).len() == 0
        })
    }

    pub fn num_mismatched_chords(&self, scale: &RootedScale) -> usize {
        self.chord_iter()
            .filter(|chord| chord.missing_chord_tones_from(&scale).len() > 0)
            .count()
    }

    pub fn scale_mismatches_for(&self) -> Vec<(usize, RootedScale)> {
        let mut result = vec![];
        for scale in all_rooted_scales() {
            if self.has_root_chord(&scale) {
                let mismatched = self.num_mismatched_chords(&scale);
                result.push((mismatched, scale));
            }
        }
        let weighted_roots = self.total_note_weights().ranking();
        let ranks = weighted_roots
            .iter()
            .enumerate()
            .map(|(i, n)| (*n, i))
            .collect::<HashMap<_, _>>();
        result.sort_by(|(c1, s1), (c2, s2)| {
            if *c1 < *c2 {
                Ordering::Less
            } else if *c1 > *c2 {
                Ordering::Greater
            } else {
                ranks
                    .get(&s1.root_name())
                    .unwrap()
                    .cmp(ranks.get(&s2.root_name()).unwrap())
            }
        });
        result
    }

    pub fn closest_matching_scales(&self) -> Vec<RootedScale> {
        let mismatches = self.scale_mismatches_for();
        let min_miss = mismatches[0].0;
        mismatches
            .iter()
            .take_while(|(c, _)| *c == min_miss)
            .map(|(_, rs)| rs.clone())
            .collect()
    }

    pub fn total_note_weights(&self) -> HashHistogram<NoteName, f64> {
        let mut note_histogram = HashHistogram::new();
        for i in 0..self.len() {
            let next_start = if i < self.len() - 1 {
                self.chords_starts[i + 1].1
            } else {
                self.duration
            };
            for note in self.chords_starts[i].0.note_names() {
                note_histogram.bump_by(&note, next_start - self.chords_starts[i].1);
            }
        }
        note_histogram
    }
}

impl From<&Recording> for ChordProgression {
    fn from(recording: &Recording) -> Self {
        let mut chords_starts = vec![];
        for (chord, start, _) in PitchSequence::new(recording).chords_starts_durations() {
            let push = chords_starts
                .last()
                .map_or(true, |(last_name, _)| *last_name != chord.name());
            if push {
                chords_starts.push((chord.name(), start));
            }
        }
        Self {
            chords_starts,
            duration: recording.duration(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Melody {
    notes_starts: Vec<(Note, Timestamp)>,
    duration: TotalDuration,
}

impl From<&Recording> for Melody {
    fn from(value: &Recording) -> Self {
        Self::from(PitchSequence::new(value))
    }
}

impl From<PitchSequence> for Melody {
    fn from(value: PitchSequence) -> Self {
        let mut result = Self::default();
        for (time, msg, _) in value.seq.iter() {
            if let Some((pitch, velocity)) = note_velocity_from(msg) {
                if velocity > 0 {
                    result.push(Note::new(pitch, velocity), *time);
                } else {
                    let end = result.len() - 1;
                    let (last, last_time) = &mut result.notes_starts[end];
                    let last_duration = *time - *last_time;
                    last.set_duration(last_duration);
                    result.duration = *time + DURATION_BUFFER;
                }
            }
        }
        result
    }
}

impl Melody {
    pub fn from_file(filename: &str) -> anyhow::Result<Self> {
        let r = Recording::from_file(filename)?;
        Ok(Self::from(&r))
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.notes_starts.len()
    }

    pub fn consolidated_len(&self) -> usize {
        ConsolidatedIter::new(self).count()
    }

    pub fn nth_consolidated(&self, n: usize) -> usize {
        ConsolidatedIter::new(self)
        .skip(n)
        .next()
        .unwrap()
        .0
    }

    pub fn push(&mut self, note: Note, starts_at: Timestamp) {
        self.notes_starts.push((note, starts_at));
        self.duration = starts_at + note.duration();
    }

    pub fn pop(&mut self) -> Option<(Note, Timestamp)> {
        self.notes_starts.pop()
    }

    pub fn starts_notes_lens(&'_ self) -> ConsolidatedIter<'_> {
        ConsolidatedIter::new(self)
    }

    pub fn starts_notes_lens_reverse(
        &'_ self,
        start: usize,
    ) -> impl Iterator<Item = (usize, u8, usize)> {
        ConsolidatedIter::new_from(self, start)
        .rev()
    }

    pub fn next_note_time(&self, i: usize) -> Timestamp {
        if i == self.len() {
            self.duration
        } else {
            self[i].1
        }
    }

    pub fn midi(&self) -> Vec<(Timestamp, MidiMsg)> {
        let mut result = vec![];
        for (i, (note, time)) in self.iter().enumerate() {
            let (on, off) = note.midi_on_off();
            result.push((*time, on));
            let note_off_timestamp = self.next_note_time(i) - (*time + note.duration());
            result.push((note_off_timestamp, off))
        }
        result
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Note, Timestamp)> {
        self.notes_starts.iter()
    }

    pub fn iter_direction(&self) -> NoteDirectionIter<'_> {
        NoteDirectionIter {
            direction: MelodyDirection::Ascending,
            melody: self,
            i: 0,
        }
    }

    pub fn duration(&self) -> TotalDuration {
        self.duration
    }

    pub fn min_max_pitches(&self) -> Option<(u8, u8)> {
        let mut iter = self.iter();
        if let Some(first) = iter.next() {
            let mut min = first.0.pitch();
            let mut max = min;
            for (note, _) in iter {
                if note.pitch() < min {
                    min = note.pitch();
                }
                if note.pitch() > max {
                    max = note.pitch();
                }
            }
            Some((min, max))
        } else {
            None
        }
    }

    pub fn highest_weight_scale(&self) -> RootedScale {
        all_rooted_scales()
            .map(|scale| (scale.clone(), self.scale_score(&scale)))
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(scale, _)| scale)
            .unwrap()
    }

    pub fn scale_score(&self, scale: &RootedScale) -> f64 {
        let mut total_weight = 0.0;
        for (note, direction) in self.iter_direction() {
            let symbol = match direction {
                MelodyDirection::Ascending => scale.ascending_note_weight(note.pitch()),
                MelodyDirection::Descending => scale.descending_note_weight(note.pitch()),
            };
            total_weight += note.duration() * symbol.map_or(-1.0, |(_, w)| w);
        }
        total_weight
    }

    pub fn without_ghosts(&self, longest_ghost: NoteDuration) -> Self {
        let mut notes = vec![];
        let mut ghostliness = 0.0;
        for (n, start) in self.iter() {
            if n.duration() > longest_ghost {
                let mut n = *n;
                if ghostliness > 0.0 {
                    n.set_duration(n.duration() + ghostliness);
                    ghostliness = 0.0;
                }
                notes.push((n, *start));
            } else {
                ghostliness += n.duration();
            }
        }
        Self {
            notes_starts: notes,
            duration: self.duration,
        }
    }

    pub fn mean_preceding_duration(&self, i: usize) -> Option<NoteDuration> {
        if i < DURATION_MOVING_WINDOW_SIZE || i >= self.len() {
            None
        } else {
            Some(
                ((i - DURATION_MOVING_WINDOW_SIZE)..i)
                    .map(|n| self[n].0.duration())
                    .sum::<NoteDuration>()
                    / DURATION_MOVING_WINDOW_SIZE as NoteDuration,
            )
        }
    }

    pub fn phrase_starts_at(&self, i: usize) -> bool {
        i == 0 || self.phrase_ends_at(i - 1)
    }

    pub fn phrase_ends_at(&self, i: usize) -> bool {
        i + 1 == self.len()
            || self.mean_preceding_duration(i).map_or(false, |m| {
                self[i].0.duration() > PHRASE_ENDING_DURATION_MULTIPLIER * m
            })
    }

    pub fn distinct_pitch_segment(&self, start: usize, len: usize) -> Vec<u8> {
        ConsolidatedIter::new_from(self, start)
        .take(min(len, self.len() - start))
        .map(|(_, p, _)| p)
        .collect()
    }
}

pub struct ConsolidatedIter<'a> {
    start: usize,
    len_forward: usize,
    after_end: usize,
    melody: &'a Melody,
}

impl<'a> ConsolidatedIter<'a> {
    fn new(melody: &'a Melody) -> Self {
        Self {
            start: 0,
            len_forward: 1,
            after_end: melody.len(),
            melody
        }
    }

    fn new_from(melody: &'a Melody, start: usize) -> Self {
        Self {
            start,
            len_forward: 1,
            after_end: start + 1,
            melody,
        }
    }

    fn pitch(&self) -> u8 {
        self.melody[self.start].0.pitch()
    }

    fn end(&self) -> usize {
        self.start + self.len_forward
    }
}

impl<'a> Iterator for ConsolidatedIter<'a> {
    type Item = (usize, u8, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.melody.len() {
            while self.end() < self.melody.len()
                && self.melody[self.end()].0.pitch() == self.pitch()
            {
                self.len_forward += 1;
            }
            let result = (self.start, self.pitch(), self.len_forward);
            self.start += self.len_forward;
            self.len_forward = 1;
            Some(result)
        } else {
            None
        }
    }
}

impl<'a> DoubleEndedIterator for ConsolidatedIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.after_end > 0 {
            let prev = self.after_end - 1;
            let mut i = prev;
            loop {
                if self.melody[i].0.pitch() != self.melody[prev].0.pitch() {
                    let start = i + 1;
                    let result = (start, self.melody[prev].0.pitch(), self.after_end - start);
                    self.after_end = start;
                    return Some(result);
                }
                if i > 0 {
                    i -= 1;
                } else {
                    let result = (0, self.melody[0].0.pitch(), self.after_end);
                    self.after_end = 0;
                    return Some(result);
                }
            }
        } else {
            None
        }
    }
}

impl Index<usize> for Melody {
    type Output = (Note, Timestamp);

    fn index(&self, index: usize) -> &Self::Output {
        &self.notes_starts[index]
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MelodyDirection {
    Ascending,
    Descending,
}

pub struct NoteDirectionIter<'a> {
    direction: MelodyDirection,
    melody: &'a Melody,
    i: usize,
}

impl<'a> Iterator for NoteDirectionIter<'a> {
    type Item = (&'a Note, MelodyDirection);

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.melody.notes_starts.len() {
            None
        } else {
            if self.i > 0 {
                let prev_pitch = self.melody.notes_starts[self.i - 1].0.pitch();
                let current_pitch = self.melody.notes_starts[self.i].0.pitch();
                if prev_pitch < current_pitch {
                    self.direction = MelodyDirection::Ascending;
                } else if prev_pitch > current_pitch {
                    self.direction = MelodyDirection::Descending;
                }
            }
            let result = Some((&self.melody.notes_starts[self.i].0, self.direction));
            self.i += 1;
            result
        }
    }
}

impl From<Melody> for Recording {
    fn from(value: Melody) -> Self {
        let mut timestamp = 0.0;
        let mut result = Recording::default();
        for (duration, midi) in value.midi() {
            result.add_message(timestamp, &midi);
            timestamp += duration;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use midi_note_recorder::Recording;

    use crate::analyzer::ChordProgression;
    use crate::analyzer::Melody;
    use crate::chords::ChordMode;
    use crate::chords::ChordName;
    use crate::notes::NoteName;

    use crate::notes::Accidental::Flat as F;
    use crate::notes::Accidental::Natural as N;
    use crate::notes::Accidental::Sharp as S;
    use crate::notes::NoteLetter as NL;
    use crate::scales::ScaleMode as SM;

    #[test]
    fn test_progressions() {
        for (filename, closest) in [
            (
                "healing4",
                vec![
                    (1, SM::Major, NoteName::new(NL::E, N)),
                    (1, SM::Mixolydian, NoteName::new(NL::B, N)),
                    (1, SM::Lydian, NoteName::new(NL::A, N)),
                ],
            ),
            (
                "take5",
                vec![
                    (0, SM::Minor, NoteName::new(NL::B, F)),
                    (0, SM::Phrygian, NoteName::new(NL::B, F)),
                    (0, SM::HarmonicMinor, NoteName::new(NL::B, F)),
                    (0, SM::MelodicMinor, NoteName::new(NL::B, F)),
                    (0, SM::Minor, NoteName::new(NL::E, F)),
                    (0, SM::Dorian, NoteName::new(NL::E, F)),
                    (0, SM::MelodicMinor, NoteName::new(NL::E, F)),
                ],
            ),
            (
                "SimpleA",
                vec![
                    (0, SM::Major, NoteName::new(NL::A, N)),
                    (0, SM::Lydian, NoteName::new(NL::A, N)),
                    (0, SM::Major, NoteName::new(NL::E, N)),
                    (0, SM::Mixolydian, NoteName::new(NL::E, N)),
                    (0, SM::Minor, NoteName::new(NL::F, S)),
                    (0, SM::Dorian, NoteName::new(NL::F, S)),
                    (0, SM::MelodicMinor, NoteName::new(NL::F, S)),
                ],
            ),
        ] {
            let recording: Recording = Recording::from_file(filename).unwrap();
            let progression = ChordProgression::from(&recording);
            let mismatches = progression.scale_mismatches_for();
            let readable = mismatches
                .iter()
                .map(|(c, r)| (*c, r.mode(), r.root_name()))
                .collect::<Vec<_>>();
            let closest_match = progression.closest_matching_scales();
            assert_eq!(closest.len(), closest_match.len());

            for i in 0..closest.len() {
                assert_eq!(closest[i], readable[i]);
                assert_eq!(closest[i].1, closest_match[i].mode());
                assert_eq!(closest[i].2, closest_match[i].root_name());
            }
        }
    }

    #[test]
    fn test_chord_at_time() {
        for (filename, times, goals) in
            [("healing4", vec![1.0], vec![(NL::A, N, ChordMode::Major)])]
        {
            let recording: Recording = Recording::from_file(filename).unwrap();
            let progression = ChordProgression::from(&recording);
            for (time, (letter, modifier, mode)) in times.iter().zip(goals.iter()) {
                let expected_chord = ChordName::new(*letter, *modifier, *mode);
                assert_eq!(expected_chord, progression.chord_at_time(*time).unwrap());
            }
        }
    }

    #[test]
    fn test_melody_lengths() {
        for (melody_file, target) in [
            ("Aminor", 8),
            ("Blocrian", 8),
            ("Cmajor", 8),
            ("Ddorian", 8),
            ("Ephrygian", 8),
            ("Flydian", 8),
            ("Gmixolydian", 8),
        ] {
            let melody = Melody::from_file(melody_file).unwrap();
            println!("{melody_file}");
            assert_eq!(melody.len(), target);
        }
    }

    #[test]
    fn test_melody_scales() {
        for (melody_file, root, acc, mode) in [
            ("Aminor", NL::A, N, SM::Minor),
            ("Blocrian", NL::B, N, SM::Locrian),
            ("Cmajor", NL::C, N, SM::Major),
            ("Ddorian", NL::D, N, SM::Dorian),
            ("Ephrygian", NL::E, N, SM::Phrygian),
            ("Flydian", NL::F, N, SM::Lydian),
            ("Gmixolydian", NL::G, N, SM::Mixolydian),
            ("AMelodicMinor", NL::A, N, SM::MelodicMinor),
        ] {
            let melody = Melody::from_file(melody_file).unwrap();
            let highest_scale = melody.highest_weight_scale();
            let expected_name = NoteName::new(root, acc);
            assert_eq!(highest_scale.mode(), mode);
            assert_eq!(highest_scale.root_name(), expected_name);
        }
    }

    #[test]
    fn test_consolidated() {
        let melody = Melody::from_file("joy_world_2").unwrap();
        let expected_consolidated = vec![
            (0, 79, 1),
            (1, 78, 1),
            (2, 76, 1),
            (3, 74, 1),
            (4, 73, 1),
            (5, 72, 1),
            (6, 71, 1),
            (7, 69, 1),
            (8, 81, 1),
            (9, 69, 1),
            (10, 67, 1),
            (11, 66, 1),
            (12, 74, 1),
            (13, 76, 2),
            (15, 78, 2),
            (17, 79, 1),
            (18, 78, 1),
            (19, 80, 1),
            (20, 79, 2),
            (22, 78, 1),
            (23, 76, 1),
            (24, 74, 2),
            (26, 72, 2),
            (28, 71, 1),
            (29, 79, 3),
            (32, 78, 1),
            (33, 76, 1),
            (34, 75, 1),
            (35, 74, 1),
            (36, 75, 1),
            (37, 74, 1),
            (38, 72, 1),
            (39, 71, 1),
            (40, 83, 1),
            (41, 71, 6),
            (47, 72, 2),
            (49, 74, 1),
            (50, 73, 1),
            (51, 72, 2),
            (53, 71, 1),
            (54, 70, 1),
            (55, 69, 3),
            (58, 71, 2),
            (60, 72, 2),
            (62, 71, 2),
            (64, 67, 1),
            (65, 80, 1),
            (66, 79, 1),
            (67, 76, 1),
            (68, 74, 1),
            (69, 72, 1),
            (70, 71, 1),
            (71, 72, 1),
            (72, 71, 1),
            (73, 69, 1),
            (74, 67, 1),
        ];
        let consolidated = melody.starts_notes_lens().collect::<Vec<_>>();
        assert_eq!(consolidated, expected_consolidated);

        let mut consolidated_rev = melody.starts_notes_lens().rev().collect::<Vec<_>>();
        consolidated_rev.reverse();
        assert_eq!(consolidated_rev, expected_consolidated);
    }

    #[test]
    fn view_melody() {
        let melody = Melody::from_file("joy_world_2")
            .unwrap()
            .without_ghosts(0.05);
        let notes = melody
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (i, n.pitch(), n.duration()))
            .collect::<Vec<_>>();
        for (i, n, d) in notes.iter() {
            let phrase_end = if melody.phrase_ends_at(*i) { "*" } else { "" };
            println!("{i}: {n} {d:.3}{phrase_end}");
        }
        for (i, (d, m)) in melody.midi().iter().enumerate() {
            println!("{i}: {m:?} {d:.2}");
        }
    }

    #[test]
    fn vec_iter_rev_demo() {
        // The purpose of this test is to demonstrate the intended semantics of 
        // reversal iterators in Rust, in order to guide my design of my own.
        // Note that the one-and-same iterator separately tracks forward and reverse
        // movement. This isn't what I expected - I figured that they were consolidated.
        let v = vec!["a", "b", "c", "d", "e"];
        let mut viter = v.iter();
        assert_eq!("a", *viter.next().unwrap());
        assert_eq!("e", *viter.next_back().unwrap());
        assert_eq!("b", *viter.next().unwrap());
        assert_eq!("d", *viter.next_back().unwrap());
    }
}
