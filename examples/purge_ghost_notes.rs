use midi_note_recorder::Recording;
use music_analyzer_generator::PitchSequence;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 4 {
        println!("Usage: duration_print input_filename min_note_duration output_filename")
    }
    let recording = Recording::from_file(args[1].as_str())?;
    let recording = PitchSequence::new(&recording)
        .without_notes_below(args[2].parse::<f64>()?)
        .recording();
    recording.to_file(args[3].as_str())
}
