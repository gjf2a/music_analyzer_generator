use midi_note_recorder::Recording;
use music_analyzer_generator::{
    consolidated_times, durations_notes_from, partitioned_melody, NoteName,
};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: duration_print filename")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    let durations_notes = durations_notes_from(&recording);

    let c = consolidated_times(&durations_notes);
    println!("num consolidated: {}", c.len());

    let p = partitioned_melody(&c, 3);
    for interval in p.iter() {
        for i in interval.iter() {
            println!(
                "{:.2}\t{}\t{}\t{}",
                c[i].0,
                c[i].1,
                NoteName::name_of(c[i].1),
                c[i].2
            );
        }
        println!();
    }

    Ok(())
}
