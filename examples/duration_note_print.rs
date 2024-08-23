use midi_note_recorder::{note_velocity_from, Recording};
use music_analyzer_generator::durations_notes_from;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: duration_print filename")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    for (d, msg) in durations_notes_from(&recording) {
        if let Some((n, v)) = note_velocity_from(&msg) {
            println!("{d:.2}\t{n}\t{v}");
        }
    }
    Ok(())
}
