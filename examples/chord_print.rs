use std::{fs::File, io::Read};

use midi_note_recorder::Recording;
use music_analyzer_generator::Chord;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: print_midi filename")
    }
    let recording: Recording =
        serde_json::from_str(read_file_to_string(args[1].as_str())?.as_str())?;
    for (time, chord) in Chord::chords_from(&recording) {
        println!("{time:.2}\t{chord}");
    }
    Ok(())
}

fn read_file_to_string(filename: &str) -> anyhow::Result<String> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
