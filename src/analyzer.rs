use enum_iterator::all;

use crate::{ChordName, NoteName, PitchSequence, RootedScale, ScaleMode};
use midi_note_recorder::Recording;

#[derive(Debug)]
pub struct ChordProgression {
    chords_starts: Vec<(ChordName, f64)>,
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

    pub fn num_mismatched_chords(&self, scale: &RootedScale) -> usize {
        self.chord_iter()
            .filter(|chord| chord.missing_chord_tones_from(&scale).len() > 0)
            .count()
    }

    pub fn scale_mismatches_for(&self) -> Vec<(usize, RootedScale)> {
        let mut result = vec![];
        for mode in all::<ScaleMode>() {
            for pitch in 60..72 {
                let root = NoteName::name_of(pitch);
                let scale = mode.rooted(root);
                let mismatched = self.num_mismatched_chords(&scale);
                result.push((mismatched, scale));
            }
        }
        result.sort_by_key(|(count, _)| *count);
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
        Self { chords_starts }
    }
}

mod tests {
    use midi_note_recorder::Recording;

    use crate::NoteName;
    use crate::analyzer::ChordProgression;

    use crate::Accidental::Flat as F;
    use crate::Accidental::Natural as N;
    use crate::Accidental::Sharp as S;
    use crate::ChordMode as CM;
    use crate::NoteLetter as NL;
    use crate::ScaleMode as SM;

    #[test]
    fn test_progressions() {
        for (filename, closest) in [(
            "healing4",
            vec![
                (
                    1,
                    SM::Major,
                    NoteName {
                        letter: NL::E,
                        modifier: N,
                    },
                ),
                (
                    1,
                    SM::Dorian,
                    NoteName {
                        letter: NL::F,
                        modifier: S,
                    },
                ),
                (
                    1,
                    SM::Lydian,
                    NoteName {
                        letter: NL::A,
                        modifier: N,
                    },
                ),
                (
                    1,
                    SM::Mixolydian,
                    NoteName {
                        letter: NL::B,
                        modifier: N,
                    },
                ),
                (
                    1,
                    SM::MelodicMinor,
                    NoteName {
                        letter: NL::F,
                        modifier: S,
                    },
                ),
            ],
        ),
        ("take5", vec![
            (0, SM::Major, NoteName {letter: NL::D, modifier: F}),
            (0, SM::Major, NoteName {letter: NL::F, modifier: S}),
            (0, SM::Minor, NoteName {letter: NL::E, modifier: F}),
            (0, SM::Minor, NoteName {letter: NL::B, modifier: F}),
            (0, SM::Dorian, NoteName {letter: NL::E, modifier: F}),
            (0, SM::Dorian, NoteName {letter: NL::A, modifier: F}),
            (0, SM::Phrygian, NoteName {letter: NL::F, modifier: N}),
            (0, SM::Phrygian, NoteName {letter: NL::B, modifier: F}),
            (0, SM::Lydian, NoteName {letter: NL::F, modifier: S}),
            (0, SM::Lydian, NoteName {letter: NL::B, modifier: N}),
            (0, SM::Mixolydian, NoteName {letter: NL::D, modifier: F}),
            (0, SM::Mixolydian, NoteName {letter: NL::A, modifier: F}),
            (0, SM::Locrian, NoteName {letter: NL::C, modifier: N}),
            (0, SM::Locrian, NoteName {letter: NL::F, modifier: N}),
            (0, SM::HarmonicMinor, NoteName {letter: NL::B, modifier: F}),
            (0, SM::MelodicMinor, NoteName {letter: NL::E, modifier: F}),
            (0, SM::MelodicMinor, NoteName {letter: NL::A , modifier: F}),
            (0, SM::MelodicMinor, NoteName {letter: NL::B , modifier: F}),
        ])] {
            let recording: Recording = Recording::from_file(filename).unwrap();
            let progression = ChordProgression::from(&recording);
            let mismatches = progression.scale_mismatches_for();
            let readable = mismatches
                .iter()
                .map(|(c, r)| (*c, r.mode, r.root))
                .collect::<Vec<_>>();
            let closest_match = progression.closest_matching_scales();
            assert_eq!(closest.len(), closest_match.len());

            for i in 0..closest.len() {
                assert_eq!(closest[i], readable[i]);
                assert_eq!(closest[i].1, closest_match[i].mode);
                assert_eq!(closest[i].2, closest_match[i].root);
            }
        }
    }
}
