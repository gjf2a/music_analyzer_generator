use midi_note_recorder::Recording;
use music_analyzer_generator::Chord;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: print_midi filename [-times]")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    for (time, chord) in Chord::chords_from(&recording) {
        if args.contains(&"-times".to_string()) {
            print!("{time:.2}\t");
        }
        println!("{chord}");
    }
    Ok(())
}
