use crate::Chord;
use rand::prelude::*;

pub fn make_durations_from(
    chords: &Vec<(Chord, f64, f64)>,
    duration_candidates: &Vec<Vec<f64>>,
) -> Vec<f64> {
    let mut durations_descending_order = duration_candidates
        .iter()
        .map(|v| (v.iter().sum::<f64>(), v.clone()))
        .collect::<Vec<_>>();
    durations_descending_order.sort_by(|(s1, _), (s2, _)| s2.total_cmp(&s1));

    let mut remaining_duration = chords.iter().map(|(_, _, duration)| *duration).sum::<f64>();
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

        let durations = vec![
            vec![0.952387584],
            vec![
                0.2584818809999999,
                0.2846851240000001,
                0.26013762800000007,
                0.9718328029999999,
            ],
            vec![
                0.2464403100000001,
                0.3013259480000001,
                0.28110055499999964,
                0.9628248290000005,
            ],
            vec![
                0.2466926469999997,
                0.31866079,
                0.4793542879999997,
                0.7874722380000003,
                1.1902939689999998,
            ],
            vec![0.9574171929999995],
            vec![
                0.25127932800000075,
                0.27688124300000005,
                0.2752891900000005,
                0.9434065769999993,
            ],
            vec![
                0.2390623190000003,
                0.258863096999999,
                0.2712980310000006,
                0.805076863,
            ],
            vec![0.2310342040000002, 0.30033856500000056, 0.8675643040000001],
            vec![0.48484588699999875, 0.2477089130000003, 0.5491652299999998],
        ];

        let chords = [
            (
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
                0.027812562,
                0.5163898859999999,
            ),
            (
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
                0.544202448,
                0.532492482,
            ),
            (
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
                1.07669493,
                0.01125453200000015,
            ),
            (
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
                1.087949462,
                0.08688128499999981,
            ),
            (
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
                1.174830747,
                0.5407516360000002,
            ),
            (
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
                1.715582383,
                0.3105824939999997,
            ),
            (
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
                2.026164877,
                0.384424031,
            ),
            (
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
                2.410588908,
                0.5565633810000001,
            ),
            (
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
                2.967152289,
                0.6148300089999998,
            ),
            (
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
                3.581982298,
                0.5710318000000001,
            ),
            (
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
                4.153014098,
                0.3191578540000002,
            ),
            (
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
                4.472171952,
                0.618362984,
            ),
            (
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
                5.090534936,
                0.4433799399999998,
            ),
            (
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
                5.533914876,
                0.48751889900000034,
            ),
            (
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
                6.021433775,
                0.5712048579999998,
            ),
            (
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
                6.592638633,
                0.6721238569999999,
            ),
            (
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
                7.26476249,
                0.4963099470000003,
            ),
            (
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
                7.761072437,
                0.6140961200000001,
            ),
            (
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
                8.375168557,
                0.5006997420000001,
            ),
            (
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
                8.875868299,
                0.3187903700000003,
            ),
            (
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
                9.194658669,
                0.5912273979999991,
            ),
            (
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
                9.785886067,
                0.46546942800000046,
            ),
            (
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
                10.251355495,
                0.46450753799999944,
            ),
            (
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
                10.715863033,
                0.6172726490000002,
            ),
            (
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
                11.333135682,
                0.2908881310000009,
            ),
            (
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
                11.624023813,
                0.3767994729999984,
            ),
            (
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
                12.000823286,
                0.5057580780000013,
            ),
            (
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
                12.506581364,
                0.5540148509999998,
            ),
            (
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
                13.060596215,
                0.5315436819999988,
            ),
            (
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
                13.592139897,
                0.28288613200000157,
            ),
            (
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
                13.875026029,
                0.6655111159999993,
            ),
            (
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
                14.540537145,
                0.9370357909999996,
            ),
            (
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
                15.477572936,
                0.012405964000000935,
            ),
            (
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
                15.4899789,
                0.061693101999999556,
            ),
            (
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
                15.551672002,
                0.1970415239999994,
            ),
            (
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
                15.748713526,
                0.0,
            ),
        ]
        .to_vec();

        let result = make_durations_from(&chords, &durations);
        println!("{result:?}");
        // TODO: Write an assertion to test that:
        // * The sum of the result is less than the time plus duration of the last chord
        //   * Note: We don't represent when the last chord ends!
        // * That same sum is within the smallest total duration of the chord durations.
    }
}
