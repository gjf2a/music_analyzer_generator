use midi_note_recorder::Recording;
use music_analyzer_generator::durations_notes_from;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: duration_print filename")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    for d in durations_notes_from(&recording) {
        println!("{d:?}");
    }
    Ok(())
}
