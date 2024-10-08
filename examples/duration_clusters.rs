use midi_note_recorder::Recording;
use music_analyzer_generator::{consolidated_note_rest_times, duration_clusters, durations_notes_from};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: duration_print filename [-debug]")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    let durations_notes = durations_notes_from(&recording);

    let c = consolidated_note_rest_times(&durations_notes);
    println!("num consolidated: {}", c.len());

    let dc = duration_clusters(&c, 3);
    if args.contains(&"-debug".to_string()) {
        println!("{dc:?}");
    } else {
        for cl in dc {
            println!("{cl:?}");
        }
    }

    Ok(())
}
