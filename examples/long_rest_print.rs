use midi_note_recorder::Recording;
use music_analyzer_generator::durations_notes_from;

const MIN_LONG: f64 = 0.3;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: duration_print filename")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    for (d, n, v) in durations_notes_from(&recording) {
        println!("{d:.2}\t{n}\t{v}");
        if v == 0 && d > MIN_LONG {
            println!();
        }
    }
    Ok(())
}
