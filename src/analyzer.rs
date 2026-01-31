use std::{cmp::Ordering, collections::HashMap};

use hash_histogram::HashHistogram;
use midi_fundsp::note_velocity_from;

use crate::{
    ChordName, NoteName, PitchSequence,
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

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Note {
    pitch: u8,
    velocity: u8,
    duration: f64,
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
                    prev.duration = *time - start;
                    pending_start = None;
                }
            }
            if let Some((pitch, velocity)) = note_velocity_from(msg) {
                if velocity > 0 {
                    notes.push(Note {
                        pitch,
                        velocity,
                        duration: 0.0,
                    });
                    pending_start = Some(*time);
                }
            }
        }
        Self { notes }
    }
}

impl Melody {
    pub fn highest_weight_scale(&self) -> RootedScale {
        all_rooted_scales()
            .map(|scale| (scale.clone(), self.scale_score(&scale)))
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(scale, _)| scale)
            .unwrap()
    }

    pub fn scale_score(&self, scale: &RootedScale) -> f64 {
        let mut result = 0.0;
        let mut prev_pitch = None;
        for note in self.notes.iter() {
            let mut ascending = true;
            if let Some(prev_pitch) = prev_pitch {
                ascending = prev_pitch <= note.pitch;
            }

            let symbol = if ascending {
                scale.ascending_note_weight(note.pitch)
            } else {
                scale.descending_note_weight(note.pitch)
            };
            result += note.duration * symbol.map_or(-1.0, |(_,w)| w);

            if prev_pitch.is_none() || prev_pitch.unwrap() != note.pitch {
                prev_pitch = Some(note.pitch);
            }
        }
        result
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
    fn test_melody_scales() {
        for (melody_file, root, acc, mode) in [
            ("Aminor", NL::A, N, SM::Minor),
            ("Blocrian", NL::B, N, SM::Locrian),
            ("Cmajor", NL::C, N, SM::Major),
            ("Ddorian", NL::D, N, SM::Dorian),
            ("Ephrygian", NL::E, N, SM::Phrygian),
            ("Flydian", NL::F, N, SM::Lydian),
            ("Gmixolydian", NL::G, N, SM::Mixolydian),
        ] {
            let recording: Recording = Recording::from_file(melody_file).unwrap();
            let melody = Melody::from(&recording);
            let highest_scale = melody.highest_weight_scale();
            let expected_name = NoteName::new(root, acc);
            assert_eq!(highest_scale.mode(), mode);
            assert_eq!(highest_scale.root_name(), expected_name);
        }
    }
}
