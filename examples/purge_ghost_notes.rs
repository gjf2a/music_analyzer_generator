use midi_note_recorder::Recording;
use music_analyzer_generator::PitchSequence;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 5 {
        println!("Usage: duration_print input_filename min_note_duration min_velocity output_filename")
    }
    let input_filename = args[1].as_str();
    let min_duration = args[2].parse::<f64>()?;
    let min_velocity = args[3].parse::<u8>()?;
    let output_filename = args[4].as_str();

    let recording = Recording::from_file(input_filename)?;
    let recording = PitchSequence::new(&recording)
        .without_notes_below(min_duration, min_velocity)
        .recording();
    recording.to_file(output_filename)
}
