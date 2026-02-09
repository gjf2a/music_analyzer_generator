use enum_iterator::all;
use rand::seq::{IndexedRandom, SliceRandom};
use crate::{analyzer::Melody, figures::{FigureMatcher, MelodicFigure}};

pub fn generate_melody_from(src: &Melody) -> Option<Melody> {
    if src.len() < 3 {
        return None;
    }

    let mut result = Melody::new();
    let scale = src.highest_weight_scale();
    let starter = random_figure();
    for (i, note) in starter.projected_notes_from(src[0].pitch(), &scale).iter().enumerate() {
        result.push(src[i].repitched(*note));
    }
    let mut offset_intervals = (1..=7).flat_map(|n| [true, false].into_iter().map(move |b| (n, b))).collect::<Vec<_>>();
    offset_intervals.push((0, false));
    for i in (result.len() - 1)..src.len() {
        let current = result[i].pitch();
        offset_intervals.shuffle(&mut rand::rng());
        for (interval, up) in offset_intervals.iter() {
            let candidate = if *up {scale.note_up(current, *interval)} else {scale.note_down(current, *interval)};
            result.push(src[i].repitched(candidate.unwrap()));
            if FigureMatcher::all_notes_matching(&result) {
                break;
            } else {
                result.pop();
            }
        }
    }
    Some(result)
}

pub fn random_figure() -> MelodicFigure {
    let figures = all::<MelodicFigure>().collect::<Vec<_>>();
    let mut rng = rand::rng();
    figures.choose(&mut rng).copied().unwrap()
}