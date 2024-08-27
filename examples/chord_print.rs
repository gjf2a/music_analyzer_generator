use midi_note_recorder::Recording;
use music_analyzer_generator::PitchSequence;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: chord_print filename [-times]")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    for (time, chord) in PitchSequence::from(&recording).chords() {
        if args.contains(&"-times".to_string()) {
            print!("{time:.2}\t");
        }
        println!("{chord}");
    }
    Ok(())
}
