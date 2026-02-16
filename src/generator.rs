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
        if con_result + 2 >= con_src {
            let slack = con_src - con_result;
            let start = result.nth_consolidated(con_result - 3 + slack);
            let fig = random_figure_at_to(start, src[src.len() - 1].0.pitch(), 3, src, &scale);
            let projection = fig.projected_notes_from(result[result.len() - 1].0.pitch(), &scale);
            add_projection_to(&projection[projection.len() - slack..], &mut result, src);
        } else {
            let fig = random_figure();
            let projection = fig.projected_notes_from(result[result.len() - 1].0.pitch(), &scale);
            add_projection_to(&projection[1..], &mut result, src);
        }
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

pub fn random_figure() -> MelodicFigure {
    let figures = all::<MelodicFigure>().collect::<Vec<_>>();
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
    let mut rng = rand::rng();
    figures.choose(&mut rng).copied().unwrap()
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
        assert_eq!(melody.duration(), generated.duration());
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
