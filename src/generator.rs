use crate::{
    analyzer::Melody,
    figures::{FigureMatcher, MelodicFigure},
};
use enum_iterator::all;
use rand::seq::{IndexedRandom, SliceRandom};

pub fn generate_melody_from(src: &Melody) -> Option<Melody> {
    if src.len() < 3 {
        return None;
    }
    let mut result = Melody::new();
    let scale = src.highest_weight_scale();
    let mut starting_pitch = src[0].pitch();
    result.push(src[0].repitched(starting_pitch));
    while result.len() + 3 < src.len() {
        let fig = random_figure();
        let projection = fig.projected_notes_from(starting_pitch, &scale);
        add_projection_to(&projection[1..], &mut result, src);
        starting_pitch = result[result.len() - 1].pitch();
    }

    match src.len() - result.len() {
        1 => {}
        2 => {}
        _ => {}
    }
    Some(result)
}

fn add_projection_to(projection: &[u8], generated: &mut Melody, src: &Melody) {
    let start = generated.len();
    for (i, note) in projection.iter().enumerate() {
        generated.push(src[i + start].repitched(*note));
    }
}

pub fn generate_melody_idea_1(src: &Melody) -> Option<Melody> {
    if src.len() < 3 {
        return None;
    }

    let mut result = Melody::new();
    let scale = src.highest_weight_scale();
    let starter = random_figure();
    for (i, note) in starter
        .projected_notes_from(src[0].pitch(), &scale)
        .iter()
        .enumerate()
    {
        result.push(src[i].repitched(*note));
    }
    let mut offset_intervals = (1..=7)
        .flat_map(|n| [true, false].into_iter().map(move |b| (n, b)))
        .collect::<Vec<_>>();
    offset_intervals.push((0, false));
    for i in (result.len() - 1)..src.len() {
        let current = result[i].pitch();
        offset_intervals.shuffle(&mut rand::rng());
        for (interval, up) in offset_intervals.iter() {
            let candidate = if *up {
                scale.note_up(current, *interval)
            } else {
                scale.note_down(current, *interval)
            };
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

pub fn random_figure_3() -> MelodicFigure {
    let figures = all::<MelodicFigure>()
        .filter(|fig| fig.pattern().len() == 2)
        .collect::<Vec<_>>();
    let mut rng = rand::rng();
    figures.choose(&mut rng).copied().unwrap()
}

pub fn random_figure_4() -> MelodicFigure {
    let figures = all::<MelodicFigure>()
        .filter(|fig| fig.pattern().len() == 3)
        .collect::<Vec<_>>();
    let mut rng = rand::rng();
    figures.choose(&mut rng).copied().unwrap()
}
