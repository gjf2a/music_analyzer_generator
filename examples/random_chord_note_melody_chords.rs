use std::sync::{Arc, Mutex};

use crossbeam_utils::atomic::AtomicCell;
use midi_note_recorder::{Recording, stereo_playback};
use music_analyzer_generator::{
    PitchSequence, consolidated_note_rest_times, duration_clusters, durations_notes_from,
    generator::random_chord_note_melody,
};

use crossbeam_queue::SegQueue;
use midi_fundsp::{
    io::{Speaker, SynthMsg, start_output_thread},
    sounds::options,
};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: duration_print filename [-debug]")
    }
    let recording = Recording::from_file(args[1].as_str())?;
    let chords = PitchSequence::new(&recording).chords_starts_durations();

    let durations_notes = durations_notes_from(&recording);
    let c = consolidated_note_rest_times(&durations_notes);
    let dc = duration_clusters(&c, 3);

    let melody = random_chord_note_melody(&chords, &dc);
    let melody_recording = Recording::from_sequence(&melody);

    let outgoing = Arc::new(SegQueue::new());
    let program_table = Arc::new(Mutex::new(options()));
    start_output_thread::<10>(outgoing.clone(), program_table.clone());
    outgoing.push(SynthMsg::program_change(1, Speaker::Left));
    outgoing.push(SynthMsg::program_change(13, Speaker::Right));
    let playback_progress = Arc::new(AtomicCell::new(None));

    stereo_playback(
        &recording,
        &melody_recording,
        outgoing,
        |msg| SynthMsg {
            msg,
            speaker: Speaker::Left,
        },
        |msg| SynthMsg {
            msg,
            speaker: Speaker::Right,
        },
        playback_progress.clone(),
    );

    Ok(())
}
