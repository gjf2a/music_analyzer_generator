use crate::{analyzer::Melody, figures::MelodicFigure, scales::RootedScale};
use enum_iterator::all;
use rand::seq::IndexedRandom;

pub fn generate_melody_from(src: &Melody) -> Option<Melody> {
    println!("{}", src.len());
    if src.len() < 3 {
        return None;
    }
    let mut result = Melody::new();
    let scale = src.highest_weight_scale();
    result.push(src[0].0, src[0].1);
    while result.len() < src.len() {
        let con_result = result.consolidated_len();
        let con_src = src.consolidated_len();
        let slack = con_src - con_result;
        let (figure, start) =
            random_fitting_figure(&result, &scale, src.len(), src[src.len() - 1].0.pitch());
        let projection = figure.projected_notes_from(result[start].0.pitch(), &scale);
        let projection_start = result.len() - start;
        add_projection_to(&projection[projection_start..], &mut result, src);
    }
    Some(result)
}

fn add_projection_to(projection: &[u8], generated: &mut Melody, src: &Melody) {
    let start = generated.len();
    let mut mi = start;
    for note in projection.iter() {
        let melody_note = src[mi].0.pitch();
        loop {
            generated.push(src[mi].0.repitched(*note), src[mi].1);
            mi += 1;
            if mi == src.len() || src[mi].0.pitch() != melody_note {
                break;
            }
        }
    }
}

pub fn random_figure(
    starting_pitch: u8,
    scale: &RootedScale,
    min_pitch: u8,
    max_pitch: u8,
) -> MelodicFigure {
    let figures = all::<MelodicFigure>()
        .filter(|fig| {
            fig.projected_notes_from(starting_pitch, scale)
                .iter()
                .all(|n| min_pitch <= *n && *n <= max_pitch)
        })
        .collect::<Vec<_>>();
    let mut rng = rand::rng();
    figures.choose(&mut rng).copied().unwrap()
}

pub fn random_figure_at(
    start: usize,
    fig_notes: usize,
    melody: &Melody,
    scale: &RootedScale,
) -> MelodicFigure {
    let figures = all::<MelodicFigure>()
        .filter(|f| f.pattern().len() + 1 == fig_notes && f.fits_at(melody, scale, start))
        .collect::<Vec<_>>();
    let mut rng = rand::rng();
    figures.choose(&mut rng).copied().unwrap()
}

pub fn random_figure_at_to(
    start: usize,
    target_pitch: u8,
    fig_notes: usize,
    melody: &Melody,
    scale: &RootedScale,
) -> MelodicFigure {
    let figures = all::<MelodicFigure>()
        .filter(|f| {
            f.pattern().len() + 1 == fig_notes && f.fits_ends_at(melody, scale, start, target_pitch)
        })
        .collect::<Vec<_>>();
    let projections = figures
        .iter()
        .map(|fig| fig.projected_notes_from(melody[start].0.pitch(), scale))
        .collect::<Vec<_>>();
    println!("target: {target_pitch}: {projections:?}");
    let mut rng = rand::rng();
    figures.choose(&mut rng).copied().unwrap()
}

pub fn random_fitting_figure(
    melody: &Melody,
    scale: &RootedScale,
    target_len: usize,
    ending_pitch: u8,
) -> (MelodicFigure, usize) {
    let candidates = all_fitting_figures(melody, scale, target_len, ending_pitch);
    let mut rng = rand::rng();
    candidates.choose(&mut rng).copied().unwrap()
}

pub fn all_fitting_figures(
    melody: &Melody,
    scale: &RootedScale,
    target_len: usize,
    ending_pitch: u8,
) -> Vec<(MelodicFigure, usize)> {
    let mut result = vec![];
    for fig in all::<MelodicFigure>() {
        for backup in 0..fig.pattern().len() {
            if melody.len() >= backup + 1 {
                let start = melody.len() - backup - 1;

                if start + fig.pattern().len() < target_len {
                    if fig.fits_at(melody, scale, start) {
                        result.push((fig, start));
                    }
                } else if start + fig.pattern().len() == target_len {
                    if fig.fits_ends_at(melody, scale, start, ending_pitch) {
                        result.push((fig, start));
                    }
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::{analyzer::Melody, generator::generate_melody_from};

    #[test]
    fn test_generator() {
        let melody = Melody::from_file("joy_world_2")
            .unwrap()
            .without_ghosts(0.05);
        let generated = generate_melody_from(&melody).unwrap();
        assert_eq!(melody.len(), generated.len());
        //assert_eq!(melody.duration(), generated.duration());
        assert_eq!(melody[0], generated[0]);
        println!(
            "{:?} -> {:?}",
            melody[melody.len() - 1],
            generated[generated.len() - 1]
        );
        assert!(
            melody[melody.len() - 1]
                .0
                .octave_equivalent(generated[generated.len() - 1].0)
        );
    }
}
