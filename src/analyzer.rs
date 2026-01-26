use enum_iterator::all;

use crate::{ChordName, NoteName, PitchSequence, RootedScale, ScaleMode};
use midi_note_recorder::Recording;

pub struct ChordProgression {
    chords_starts: Vec<(ChordName, f64)>
}

impl ChordProgression {
    pub fn chord_iter(&self) -> impl Iterator<Item=ChordName> {
        self.chords_starts.iter().map(|(c,_)| *c)
    }

    pub fn chord_start_iter(&self) -> impl Iterator<Item=(ChordName, f64)> {
        self.chords_starts.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.chords_starts.len()
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
        Self {chords_starts}
    }
}

pub fn num_mismatched_chords<P: Iterator<Item = ChordName>>(
    scale: &RootedScale,
    progression: P,
) -> usize {
    progression
        .filter(|chord| chord.missing_chord_tones_from(&scale).len() > 0)
        .count()
}

pub fn scale_mismatches_for<P: Iterator<Item = ChordName>>(progression: P) -> Vec<(usize,RootedScale)> {
    let mut result = vec![];
    let progression = progression.collect::<Vec<_>>();
    for mode in all::<ScaleMode>() {
        for pitch in 60..72 {
            let root = NoteName::name_of(pitch);
            let scale = mode.rooted(root);
            let mismatched = num_mismatched_chords(&scale, progression.iter().copied());
            result.push((mismatched, scale));
        }
    }
    result.sort_by_key(|(count,_)| *count);
    result
}

mod tests {

}
