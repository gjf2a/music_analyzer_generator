use crate::{analyzer::Melody, figures::MelodicFigure, notes::octave_equivalent, scales::RootedScale};
use enum_iterator::all;
use rand::seq::IndexedRandom;

pub fn generate_melody_from(src: &Melody) -> Option<Melody> {
    let mut rng = rand::rng();
    println!("{}", src.len());
    if src.len() < 3 {
        return None;
    }
    let mut result = Melody::new();
    let scale = src.highest_weight_scale();
    result.push(src[0].0, src[0].1);
    while result.len() < src.len() {
        let candidates = all_possible_extensions_for(&result, src, &scale);
        let mut choice = candidates.choose(&mut rng).unwrap().clone();
        std::mem::swap(&mut choice, &mut result);
    }
    Some(result)
}

fn add_projection_to(projection: &[u8], generated: &Melody, src: &Melody) -> Option<Melody> {
    let mut result = generated.clone();
    let start = generated.len();
    let mut mi = start;
    for note in projection.iter() {
        if mi >= src.len() {
            return None;
        }
        let melody_note = src[mi].0.pitch();
        loop {
            result.push(src[mi].0.repitched(*note), src[mi].1);
            mi += 1;
            if mi == src.len() || src[mi].0.pitch() != melody_note {
                break;
            }
        }
    }
    if result.len() == src.len() && !octave_equivalent(result[result.len() - 1].0.pitch(), src[src.len() - 1].0.pitch()) {
        None
    } else {
        Some(result)
    }
}

pub fn all_possible_extensions_for(target: &Melody, src: &Melody, scale: &RootedScale) -> Vec<Melody> {
    let mut possible = vec![];
    for fig in all::<MelodicFigure>() {
        for ((start, starting_pitch, _), p) in target.starts_notes_lens().rev().zip(0..fig.pattern().len()) {
            if fig.fits_at(target, scale, start) {
                let projection = fig.projected_notes_from(starting_pitch, scale);
                if let Some(candidate) = add_projection_to(&projection[p..], &target, src) {
                    possible.push(candidate);
                }
            }
        }
    }
    possible
}

#[cfg(test)]
mod tests {
    use crate::{analyzer::Melody, generator::generate_melody_from};

    // TODO: Write unit tests for add_projection_to() and all_possible_extensions_for()

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
