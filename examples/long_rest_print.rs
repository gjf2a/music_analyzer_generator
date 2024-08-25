use midi_note_recorder::Recording;
use music_analyzer_generator::{durations_notes_from, partitioned_melody};

const MIN_LONG: f64 = 0.3;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        println!("Usage: duration_print filename")
    }
    let recording: Recording = Recording::from_file(args[1].as_str())?;
    let durations_notes = durations_notes_from(&recording);

    println!("Median rest time: {}", median_rest_time(&durations_notes));
    println!("Mean rest time:   {}", mean_rest_time(&durations_notes));

    for (d, n, v) in durations_notes.iter() {
        println!("{d:.2}\t{n}\t{v}");
        if *v == 0 && *d > MIN_LONG {
            println!();
        }
    }

    println!();
    println!();
    println!();

    let c = consolidated_times(&durations_notes);
    println!("num consolidated: {}", c.len());

    for (d, n, v) in c.iter() {
        println!("{d:.2}\t{n}\t{v}");
        if *v == 0 && *d > MIN_LONG {
            println!();
        }
    }

    println!();
    println!();
    println!("Partitioned");
    let p = partitioned_melody(&c, 3);
    for interval in p.iter() {
        for i in interval.iter() {
            println!("{:.2}\t{}\t{}", c[i].0, c[i].1, c[i].2);
        }
        println!();
    }

    Ok(())
}

fn rest_times(durations_notes: &Vec<(f64, u8, u8)>) -> Vec<f64> {
    durations_notes
        .iter()
        .filter(|(_, _, v)| *v == 0)
        .map(|(t, _, _)| *t)
        .collect::<Vec<_>>()
}

fn median_rest_time(durations_notes: &Vec<(f64, u8, u8)>) -> f64 {
    let mut rest_times = rest_times(durations_notes);
    rest_times.sort_by(|f1, f2| f1.partial_cmp(f2).unwrap());
    rest_times[rest_times.len() / 2]
}

fn mean_rest_time(durations_notes: &Vec<(f64, u8, u8)>) -> f64 {
    let rt = rest_times(durations_notes);
    rt.iter().sum::<f64>() / rt.len() as f64
}

fn consolidated_times(durations_notes: &Vec<(f64, u8, u8)>) -> Vec<(f64, u8, u8)> {
    (0..durations_notes.len())
        .step_by(2)
        .map(|i| {
            (
                durations_notes[i].0 + durations_notes[i + 1].0,
                durations_notes[i].1,
                durations_notes[i].2,
            )
        })
        .collect()
}
