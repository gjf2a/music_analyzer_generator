use std::{cmp::Ordering, collections::HashMap, ops::Index};

use hash_histogram::HashHistogram;
use midi_fundsp::note_velocity_from;

use crate::{
    ChordName, NoteName, PitchSequence,
    notes::Note,
    scales::{RootedScale, all_rooted_scales},
};
use midi_note_recorder::Recording;

#[derive(Debug)]
pub struct ChordProgression {
    chords_starts: Vec<(ChordName, f64)>,
    duration: f64,
}

impl ChordProgression {
    pub fn chord_iter(&self) -> impl Iterator<Item = ChordName> {
        self.chords_starts.iter().map(|(c, _)| *c)
    }

    pub fn chord_start_iter(&self) -> impl Iterator<Item = (ChordName, f64)> {
        self.chords_starts.iter().copied()
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

#[derive(Clone, Debug, PartialEq)]
pub struct Melody {
    notes: Vec<Note>,
}

impl From<&Recording> for Melody {
    fn from(value: &Recording) -> Self {
        Self::from(PitchSequence::new(value))
    }
}

impl From<PitchSequence> for Melody {
    fn from(value: PitchSequence) -> Self {
        let mut notes: Vec<Note> = vec![];
        let mut pending_start = None;
        for (time, msg, _) in value.seq.iter() {
            if let Some(prev) = notes.last_mut() {
                if let Some(start) = pending_start {
                    prev.set_duration(*time - start);
                    pending_start = None;
                }
            }
            if let Some((pitch, velocity)) = note_velocity_from(msg) {
                if velocity > 0 {
                    notes.push(Note::new(pitch, velocity));
                    pending_start = Some(*time);
                }
            }
        }
        Self { notes }
    }
}

impl Melody {
    pub fn from_file(filename: &str) -> anyhow::Result<Self> {
        let r = Recording::from_file(filename)?;
        Ok(Self::from(&r))
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn starts_notes_lens(&'_ self) -> ConsolidatedIter<'_> {
        ConsolidatedIter {
            start: 0,
            len: 1,
            melody: self,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Note> {
        self.notes.iter()
    }

    pub fn iter_direction(&self) -> NoteDirectionIter<'_> {
        NoteDirectionIter {
            direction: MelodyDirection::Ascending,
            melody: self,
            i: 0,
        }
    }

    pub fn duration(&self) -> f64 {
        self.iter().map(|n| n.duration()).sum()
    }

    pub fn min_max_pitches(&self) -> Option<(u8, u8)> {
        let mut iter = self.iter();
        if let Some(first) = iter.next() {
            let mut min = first.pitch();
            let mut max = min;
            for note in iter {
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

    pub fn without_ghosts(&self, longest_ghost: f64) -> Self {
        let mut notes = vec![];
        let mut ghostliness = 0.0;
        for n in self.iter() {
            if n.duration() > longest_ghost {
                let mut n = *n;
                if ghostliness > 0.0 {
                    n.set_duration(n.duration() + ghostliness);
                    ghostliness = 0.0;
                }
                notes.push(n);
            } else {
                ghostliness += n.duration();
            }
        }
        Self { notes }
    }
}

pub struct ConsolidatedIter<'a> {
    start: usize,
    len: usize,
    melody: &'a Melody,
}

impl<'a> ConsolidatedIter<'a> {
    fn pitch(&self) -> u8 {
        self.melody[self.start].pitch()
    }

    fn end(&self) -> usize {
        self.start + self.len
    }
}

impl<'a> Iterator for ConsolidatedIter<'a> {
    type Item = (usize, u8, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.melody.len() {
            while self.end() < self.melody.len() && self.melody[self.end()].pitch() == self.pitch()
            {
                self.len += 1;
            }
            let result = (self.start, self.pitch(), self.len);
            self.start += self.len;
            self.len = 1;
            Some(result)
        } else {
            None
        }
    }
}

impl Index<usize> for Melody {
    type Output = Note;

    fn index(&self, index: usize) -> &Self::Output {
        &self.notes[index]
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
        if self.i >= self.melody.notes.len() {
            None
        } else {
            if self.i > 0 {
                let prev_pitch = self.melody.notes[self.i - 1].pitch();
                let current_pitch = self.melody.notes[self.i].pitch();
                if prev_pitch < current_pitch {
                    self.direction = MelodyDirection::Ascending;
                } else if prev_pitch > current_pitch {
                    self.direction = MelodyDirection::Descending;
                }
            }
            let result = Some((&self.melody.notes[self.i], self.direction));
            self.i += 1;
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use midi_note_recorder::Recording;

    use crate::analyzer::ChordProgression;
    use crate::analyzer::Melody;
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
    }

    #[test]
    fn view_melody() {
        let melody = Melody::from_file("joy_world_2")
            .unwrap()
            .without_ghosts(0.05);
        let notes = melody
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.pitch(), n.duration()))
            .collect::<Vec<_>>();
        for (i, n, d) in notes {
            println!("{i}: {n} {d:.3}");
        }
    }
}
