use enum_iterator::all;

use crate::{ChordName, NoteName, PitchSequence, RootedScale, ScaleMode};
use midi_note_recorder::Recording;

pub fn chords_starts(recording: &Recording) -> Vec<(ChordName, f64)> {
    let mut result = vec![];
    for (chord, start, _) in PitchSequence::new(recording).chords_starts_durations() {
        let push = result
            .last()
            .map_or(true, |(last_name, _)| *last_name != chord.name());
        if push {
            result.push((chord.name(), start));
        }
    }
    result
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

mod tests {}
