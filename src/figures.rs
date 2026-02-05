use std::{cmp::min, collections::BTreeSet, fmt::Display};

use enum_iterator::{Sequence, all};

use crate::{analyzer::Melody, scales::RootedScale};

// Inspired by: https://figuringoutmelody.com/the-24-universal-melodic-figures/
#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence, Hash, Ord, PartialOrd)]
pub struct MelodicFigure {
    shape: MelodicFigureShape,
    polarity: FigurePolarity,
    direction: FigureDirection,
}

impl Display for MelodicFigure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p = match self.polarity {
            FigurePolarity::Positive => "+",
            FigurePolarity::Negative => "-",
        };
        let d = match self.direction {
            FigureDirection::Forward => ">",
            FigureDirection::Reverse => "<",
        };
        write!(f, "{:?}{d}{p}{:?}", self.shape, self.pattern())
    }
}

// TODO from here:
//
// We can start by appending any diatonic note onto a melody.
// We then check to make sure that every note belongs to at least
// one figure.
//
// So we begin with a method that, given a Melody and index, returns true
// if there exists a Figure that includes the note at that index, and false
// otherwise. To do this, for each Figure we consider our note at each of the
// 3-4 positions within that Figure. If the Note's context is compatible with the
// Figure thusly positioned, we return true. If this never happens, we return false.
// * Caveats: There should be a number indicating how many more notes are left before
//.  finishing the melody. Any figure extending past the current note into undetermined
//.  territory should take this into account. We might even demand that we only include
//.  figures whose final note is the tonic of the scale.
//
// Next, we iterate over the Melody, and if every index returns true, it is a
// valid melody. Note that, as we incrementally build the melody, we only need to check
// the last four Notes - prior to that, everyone has already been checked.
//

impl MelodicFigure {
    pub fn matching_figures(melody: &Melody) -> Vec<(usize, BTreeSet<Self>)> {
        let mut result = vec![];
        let scale = melody.highest_weight_scale();
        let consolidated = melody.starts_notes_lens().collect::<Vec<_>>();
        for (ci, (mi, _, _)) in consolidated.iter().copied().enumerate() {
            let mut figures = BTreeSet::new();
            for figure in all::<Self>() {
                let pattern = figure.pattern();
                let back_up_start = if ci <= pattern.len() {
                    0
                } else {
                    ci - pattern.len()
                };
                let go_forward_end = min(ci + pattern.len(), consolidated.len());
                for j in back_up_start..go_forward_end {
                    if Self::pattern_aligned_at(j, &pattern, &scale, &consolidated) {
                        figures.insert(figure);
                    }
                }
            }
            result.push((mi, figures));
        }
        result
    }

    fn pattern_aligned_at(
        j: usize,
        pattern: &Vec<i16>,
        scale: &RootedScale,
        consolidated: &Vec<(usize, u8, usize)>,
    ) -> bool {
        let mut pattern_notes = vec![consolidated[j].1];
        for diatonic_steps in pattern.iter() {
            let current = pattern_notes[pattern_notes.len() - 1];
            if *diatonic_steps > 0 {
                pattern_notes.push(
                    scale
                        .note_up(current, *diatonic_steps as usize + 1)
                        .unwrap(),
                );
            } else {
                pattern_notes.push(
                    scale
                        .note_down(current, -(*diatonic_steps) as usize + 1)
                        .unwrap(),
                );
            }
        }
        (0..pattern_notes.len())
            .all(|k| k + j >= consolidated.len() || consolidated[k + j].1 == pattern_notes[k])
    }

    pub fn pattern(&self) -> Vec<i16> {
        let mut result = self.shape.pattern();
        if self.polarity == FigurePolarity::Negative {
            for n in result.iter_mut() {
                *n = -*n;
            }
        }
        if self.direction == FigureDirection::Reverse {
            result.reverse();
        }
        result
    }

    pub fn len(&self) -> usize {
        self.pattern().len() + 1
    }

    /// Returns the net change of diatonic steps from the start to the end of this `MelodicFigure`.
    pub fn total_diatonic_change(&self) -> i16 {
        self.pattern().iter().sum()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence, Hash, Ord, PartialOrd)]
pub enum MelodicFigureShape {
    Note3Scale,
    Auxiliary,
    Arpeggio,
    Run,
    Trill1,
    Trill2,
    Arch,
    NP3,
    PivotLHP,
    ReturnCrazyDriver,
    ArpeggioPlus,
    Parkour1,
    ParkourBounce2,
    ParkourPounce2,
    Vault4,
    Vault5,
    Vault6,
    Vault7,
    Roll,
    DoubleNeighbor,
    Double3rd,
    Pendulum43,
    Pendulum54,
    LeapingScale,
    LeapingAux1,
    LeapingAux2,
    PendulumAux1,
    PendulumAux2,
    Funnel,
    Cambiata1,
    Cambiata2,
    ZigZag1,
    ZigZag2,
}

impl MelodicFigureShape {
    pub fn pattern(&self) -> Vec<i16> {
        match self {
            Self::Note3Scale => vec![1, 1],
            Self::Auxiliary => vec![-1, 1],
            Self::Arpeggio => vec![2, 2],
            Self::Run => vec![1, 1, 1],
            Self::Trill1 => vec![1, -1, 1],
            Self::Trill2 => vec![2, -2, 2],
            Self::Arch => vec![2, 2, -2],
            Self::NP3 => vec![-2, -1],
            Self::PivotLHP => vec![1, -2],
            Self::ReturnCrazyDriver => vec![1, 1, -1],
            Self::ArpeggioPlus => vec![2, 2, -1],
            Self::Parkour1 => vec![-1, 3],
            Self::ParkourPounce2 => vec![1, -6],
            Self::ParkourBounce2 => vec![-5, 1],
            Self::Vault4 => vec![4, 1],
            Self::Vault5 => vec![5, 1],
            Self::Vault6 => vec![6, 1],
            Self::Vault7 => vec![1, 7],
            Self::Roll => vec![1, 1, -2],
            Self::DoubleNeighbor => vec![1, -2, 1],
            Self::Double3rd => vec![2, -1, 2],
            Self::Pendulum43 => vec![4, -3],
            Self::Pendulum54 => vec![5, -4],
            Self::LeapingScale => vec![1, 1, 2],
            Self::LeapingAux1 => vec![-1, 1, 4],
            Self::LeapingAux2 => vec![1, -1, 4],
            Self::PendulumAux1 => vec![4, -5, 1],
            Self::PendulumAux2 => vec![4, -3, -1],
            Self::Funnel => vec![3, -2, 1],
            Self::Cambiata1 => vec![1, 2, -1],
            Self::Cambiata2 => vec![1, -4, 5],
            Self::ZigZag1 => vec![4, -2, 5],
            Self::ZigZag2 => vec![-1, 5, -1],
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence, Hash, Ord, PartialOrd)]
pub enum FigurePolarity {
    Positive,
    Negative,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence, Hash, Ord, PartialOrd)]
pub enum FigureDirection {
    Forward,
    Reverse,
}

#[cfg(test)]
mod tests {
    use enum_iterator::all;

    use crate::{analyzer::Melody, figures::MelodicFigure};

    #[test]
    fn see_sequence() {
        for mf in all::<MelodicFigure>() {
            println!("{mf:?}");
        }
    }

    #[test]
    fn test_matching_figures() {
        let melody = Melody::from_file("joy_world_2")
            .unwrap()
            .without_ghosts(0.05);
        let figures = MelodicFigure::matching_figures(&melody);
        for (i, figs) in figures {
            let figstr = figs.iter().map(|f| format!("{f} ")).collect::<String>();
            println!("{i}: {} {figstr}\n", melody[i].pitch());
        }
    }
}
