use crate::Chord;
use rand::prelude::*;

pub fn make_durations_from(chords: &Vec<(f64, Chord)>, durations: &Vec<Vec<f64>>) -> Vec<f64> {
    let mut durations_descending_order = durations
        .iter()
        .map(|v| (v.iter().sum::<f64>(), v.clone()))
        .collect::<Vec<_>>();
    durations_descending_order.sort_by(|(s1, _), (s2, _)| s2.total_cmp(&s1));

    let mut remaining_duration = chords.last().map(|(t,_)| *t).unwrap();
    let mut start = 0;
    let mut rng = thread_rng();

    let mut result = vec![];
    while let Some((total, durations)) = durations_descending_order[start..].choose(&mut rng) {
        result.extend(durations.iter());
        remaining_duration -= *total;
        while start < durations_descending_order.len()
            && durations_descending_order[start].0 > remaining_duration
        {
            start += 1;
        }
    }
    result
}

pub fn make_melody_from(chords: Vec<(f64, Chord)>, durations: Vec<Vec<f64>>) -> Vec<(f64, u8, u8)> {
    todo!("Not even started")
}

#[cfg(test)]
mod tests {
    use crate::generator::make_durations_from;


    #[test]
    fn test_make_durations() {
        use crate::Accidental::*;
        use crate::ActivePitches;
        use crate::ChordMode::*;
        use crate::NoteLetter::*;
        use crate::{Chord, ChordName};

        let durations = vec![vec![0.952387584], vec![0.2584818809999999, 0.2846851240000001, 0.26013762800000007, 0.9718328029999999], vec![0.2464403100000001, 0.3013259480000001, 0.28110055499999964, 0.9628248290000005], vec![0.2466926469999997, 0.31866079, 0.4793542879999997, 0.7874722380000003, 1.1902939689999998], vec![0.9574171929999995], vec![0.25127932800000075, 0.27688124300000005, 0.2752891900000005, 0.9434065769999993], vec![0.2390623190000003, 0.258863096999999, 0.2712980310000006, 0.805076863], vec![0.2310342040000002, 0.30033856500000056, 0.8675643040000001], vec![0.48484588699999875, 0.2477089130000003, 0.5491652299999998]];

        let chords = [
            (
                0.027812562,
                Chord {
                    name: ChordName {
                        note: A,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 611048397441628897280,
                    },
                },
            ),
            (
                0.544202448,
                Chord {
                    name: ChordName {
                        note: A,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 611048397441628897280,
                    },
                },
            ),
            (
                1.07669493,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                1.087949462,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Flat,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 83875039460148117504,
                    },
                },
            ),
            (
                1.174830747,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                1.715582383,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                2.026164877,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                2.410588908,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 314171110005365800960,
                    },
                },
            ),
            (
                2.967152289,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 314171110005365800960,
                    },
                },
            ),
            (
                3.581982298,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                4.153014098,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                4.472171952,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                5.090534936,
                Chord {
                    name: ChordName {
                        note: A,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 611048397441628897280,
                    },
                },
            ),
            (
                5.533914876,
                Chord {
                    name: ChordName {
                        note: A,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 611048397441628897280,
                    },
                },
            ),
            (
                6.021433775,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                6.592638633,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                7.26476249,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 314171110005365800960,
                    },
                },
            ),
            (
                7.761072437,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 314171110005365800960,
                    },
                },
            ),
            (
                8.375168557,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                8.875868299,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                9.194658669,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                9.785886067,
                Chord {
                    name: ChordName {
                        note: A,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 611048397441628897280,
                    },
                },
            ),
            (
                10.251355495,
                Chord {
                    name: ChordName {
                        note: A,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 611048397441628897280,
                    },
                },
            ),
            (
                10.715863033,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                11.333135682,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                11.624023813,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                12.000823286,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 314171110005365800960,
                    },
                },
            ),
            (
                12.506581364,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 314171110005365800960,
                    },
                },
            ),
            (
                13.060596215,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                13.592139897,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                13.875026029,
                Chord {
                    name: ChordName {
                        note: C,
                        accidental: Sharp,
                        mode: Minor,
                    },
                    notes: ActivePitches {
                        on: 315900492262276071424,
                    },
                },
            ),
            (
                14.540537145,
                Chord {
                    name: ChordName {
                        note: A,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 20896702270999101440,
                    },
                },
            ),
            (
                15.477572936,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                15.4899789,
                Chord {
                    name: ChordName {
                        note: E,
                        accidental: Flat,
                        mode: Diminished,
                    },
                    notes: ActivePitches {
                        on: 83730924272072261632,
                    },
                },
            ),
            (
                15.551672002,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
            (
                15.748713526,
                Chord {
                    name: ChordName {
                        note: B,
                        accidental: Natural,
                        mode: Major,
                    },
                    notes: ActivePitches {
                        on: 83586809083996405760,
                    },
                },
            ),
        ].to_vec();

        let result = make_durations_from(&chords, &durations);
        println!("{result:?}");
        // TODO: Write an assertion to test that:
        // * The sum of the result is less than the time plus duration of the last chord
        //   * Note: We don't represent when the last chord ends!
        // * That same sum is within the smallest total duration of the chord durations.
    }
}
