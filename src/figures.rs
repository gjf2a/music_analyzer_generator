use std::{collections::BTreeSet, fmt::Display};

use enum_iterator::{Sequence, all};

use crate::{analyzer::Melody, notes::octave_equivalent, scales::RootedScale};

// Inspired by: https://figuringoutmelody.com/the-building-blocks-of-melody/
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
        write!(f, "{:?}{d}{p}", self.shape)
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

    pub fn projected_notes_from(&self, starting_pitch: u8, scale: &RootedScale) -> Vec<u8> {
        let mut pattern_notes = vec![starting_pitch];
        for diatonic_steps in self.pattern() {
            let current = pattern_notes[pattern_notes.len() - 1];
            if diatonic_steps > 0 {
                pattern_notes.push(scale.note_up(current, diatonic_steps as usize + 1).unwrap());
            } else {
                pattern_notes.push(
                    scale
                        .note_down(current, -(diatonic_steps) as usize + 1)
                        .unwrap(),
                );
            }
        }
        pattern_notes
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
    NotePentatonic3,
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
            Self::NotePentatonic3 => vec![-2, -1],
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

pub fn figures2string<'a, I: Iterator<Item = &'a MelodicFigure>>(figs: I) -> String {
    figs.map(|f| format!("{f} ")).collect()
}

pub struct FigureMatcher<'a> {
    melody: &'a Melody,
    scale: RootedScale,
    consolidated: Vec<(usize, u8, usize)>,
    start_table: Vec<(usize, BTreeSet<MelodicFigure>)>,
    end_table: Vec<(usize, BTreeSet<MelodicFigure>)>,
    within_table: Vec<(usize, BTreeSet<MelodicFigure>)>,
}

impl<'a> FigureMatcher<'a> {
    pub fn show_tables_for(melody: &'a Melody) {
        let matcher = Self::new(melody);
        for i in 0..matcher.consolidated.len() {
            println!(
                "{i} ({}): start: {} within: {} end: {}",
                matcher.consolidated[i].0,
                figures2string(matcher.start_table[i].1.iter()),
                figures2string(matcher.within_table[i].1.iter()),
                figures2string(matcher.end_table[i].1.iter()),
            );
        }
    }

    fn new(melody: &'a Melody) -> Self {
        let mut matcher = Self {
            melody,
            scale: melody.highest_weight_scale(),
            consolidated: melody.starts_notes_lens().collect(),
            start_table: vec![],
            end_table: vec![],
            within_table: vec![],
        };
        matcher.start_table = matcher.figure_table(Self::starts_at);
        matcher.end_table = matcher.figure_table(Self::ends_at);
        matcher.within_table = matcher.figure_table(Self::within);
        matcher
    }

    pub fn matching_figures_consolidated(melody: &'a Melody) -> Vec<(usize, BTreeSet<MelodicFigure>)> {
        let matcher = Self::new(melody);
        let mut result = vec![];
        for (ci, (mi, _, _)) in matcher.consolidated.iter().copied().enumerate() {
            result.push((mi, matcher.matching_figures_at(ci)));
        }
        result
    }

    pub fn matching_figures(melody: &'a Melody) -> Vec<BTreeSet<MelodicFigure>> {
        let mfc = Self::matching_figures_consolidated(melody);
        let mut result = vec![];
        for i in 0..mfc.len() {
            let end = if i + 1 == mfc.len() {melody.len()} else {mfc[i + 1].0};
            for _ in mfc[i].0..end {
                result.push(mfc[i].1.clone());
            }
        }
        result
    }

    fn figure_table<M: Fn(&Self, usize, MelodicFigure) -> bool>(
        &self,
        matcher: M,
    ) -> Vec<(usize, BTreeSet<MelodicFigure>)> {
        (0..self.consolidated.len())
            .map(|i| {
                (
                    i,
                    all::<MelodicFigure>()
                        .filter(|fig| matcher(self, i, *fig))
                        .collect(),
                )
            })
            .collect()
    }

    fn ends_at(&self, i: usize, fig: MelodicFigure) -> bool {
        let pattern = fig.pattern();
        if i < pattern.len() {
            false
        } else {
            self.figure_aligned_at(i - pattern.len(), fig)
        }
    }

    fn starts_at(&self, i: usize, fig: MelodicFigure) -> bool {
        self.figure_aligned_at(i, fig)
    }

    fn within(&self, i: usize, fig: MelodicFigure) -> bool {
        let pattern = fig.pattern();
        (2..=pattern.len()).map(|len| len - 1).any(|offset| {
            pattern.len() > offset && i >= offset && self.figure_aligned_at(i - offset, fig)
        })
    }

    fn figure_aligned_at(&self, figure_start: usize, figure: MelodicFigure) -> bool {
        let pattern = figure.pattern();
        if figure_start + pattern.len() >= self.consolidated.len() {
            return false;
        }
        let pattern_notes =
            figure.projected_notes_from(self.consolidated[figure_start].1, &self.scale);
        (0..pattern_notes.len()).all(|k| {
            k + figure_start >= self.consolidated.len()
                || self.consolidated[k + figure_start].1 == pattern_notes[k]
        })
    }

    fn matching_figures_at(&self, ci: usize) -> BTreeSet<MelodicFigure> {
        all::<MelodicFigure>()
            .filter(|fig| self.any_property(ci, fig))
            .collect()
    }

    fn any_property(&self, ci: usize, fig: &MelodicFigure) -> bool {
        self.start_property(ci, fig) || self.within_property(ci, fig) || self.end_property(ci, fig)
    }

    fn within_property(&self, ci: usize, fig: &MelodicFigure) -> bool {
        self.within_table[ci].1.contains(fig)
    }

    fn end_property(&self, ci: usize, fig: &MelodicFigure) -> bool {
        self.end_table[ci].1.contains(fig)
            && (self.start_table[ci].1.len() > 0
                || self.within_table[ci].1.len() > 0
                || self.melody.phrase_ends_at(self.consolidated[ci].0)
                || self.melody.phrase_ends_at(self.consolidated[ci + 1].0)
                    && octave_equivalent(self.consolidated[ci].1, self.consolidated[ci + 1].1))
    }

    fn start_property(&self, ci: usize, fig: &MelodicFigure) -> bool {
        self.start_table[ci].1.contains(fig)
            && (self.end_table[ci].1.len() > 0
                || self.within_table[ci].1.len() > 0
                || self.melody.phrase_starts_at(self.consolidated[ci].0)
                || self.end_table[ci - 1].1.len() > 0
                    && octave_equivalent(self.consolidated[ci - 1].1, self.consolidated[ci].1))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use enum_iterator::all;

    use crate::{
        analyzer::Melody,
        figures::{FigureMatcher, MelodicFigure, figures2string},
        notes::NoteName,
        scales::ScaleMode,
    };

    #[test]
    fn test_matching_figures() {
        let melody = Melody::from_file("joy_world_2")
            .unwrap()
            .without_ghosts(0.05);
        let figures = FigureMatcher::matching_figures_consolidated(&melody);
        let expected = [
            (
                0,
                79,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                1,
                78,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                2,
                76,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                3,
                74,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                4,
                72,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                5,
                71,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                6,
                69,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                7,
                67,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-", "Vault4>+"],
            ),
            (
                8,
                74,
                vec!["Note3Scale>+", "Note3Scale<+", "Run>+", "Run<+", "Vault4>+"],
            ),
            (
                9,
                76,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Run>+",
                    "Run<+",
                    "ReturnCrazyDriver>+",
                    "Vault4>+",
                ],
            ),
            (
                11,
                78,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Run>+",
                    "Run<+",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                13,
                79,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Run>+",
                    "Run<+",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                16,
                78,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                17,
                76,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                18,
                74,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                20,
                72,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "ParkourBounce2<-",
                    "ZigZag2>+",
                    "ZigZag2<+",
                ],
            ),
            (
                21,
                71,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "ParkourBounce2>-",
                    "ParkourBounce2<-",
                    "ZigZag2>+",
                    "ZigZag2<+",
                ],
            ),
            (
                22,
                79,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "ParkourBounce2>-",
                    "ParkourBounce2<-",
                    "ZigZag2>+",
                    "ZigZag2<+",
                ],
            ),
            (
                24,
                78,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "ParkourBounce2>-",
                    "ZigZag2>+",
                    "ZigZag2<+",
                ],
            ),
            (
                25,
                76,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
            (
                26,
                74,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver>-",
                ],
            ),
            (
                28,
                72,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary>+",
                    "Auxiliary<-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver<+",
                    "ReturnCrazyDriver>-",
                ],
            ),
            (
                29,
                71,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary>+",
                    "Auxiliary<-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<+",
                    "ReturnCrazyDriver>-",
                ],
            ),
            (
                35,
                72,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Auxiliary>+",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Auxiliary<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<+",
                    "ReturnCrazyDriver>-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                37,
                74,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<+",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                38,
                72,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver>-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                40,
                71,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary>+",
                    "Auxiliary<-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver<+",
                    "ReturnCrazyDriver>-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                41,
                69,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary>+",
                    "Auxiliary<-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<+",
                    "ReturnCrazyDriver>-",
                ],
            ),
            (
                44,
                71,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Auxiliary>+",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Auxiliary<-",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<+",
                    "ReturnCrazyDriver>-",
                ],
            ),
            (
                45,
                72,
                vec![
                    "Note3Scale>+",
                    "Note3Scale<+",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "NotePentatonic3<+",
                    "ReturnCrazyDriver>+",
                    "ReturnCrazyDriver<+",
                ],
            ),
            (
                47,
                71,
                vec![
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "NotePentatonic3<+",
                    "ReturnCrazyDriver>+",
                ],
            ),
            (48, 67, vec!["NotePentatonic3<+"]),
            (49, 79, vec!["NotePentatonic3>+", "LeapingScale<-"]),
            (
                50,
                76,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "NotePentatonic3>+",
                    "LeapingScale<-",
                ],
            ),
            (
                51,
                74,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "NotePentatonic3>+",
                    "ReturnCrazyDriver>-",
                    "LeapingScale<-",
                ],
            ),
            (
                52,
                72,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary>+",
                    "Auxiliary<-",
                    "Run>-",
                    "Run<-",
                    "Trill1>-",
                    "Trill1<-",
                    "ReturnCrazyDriver>-",
                    "LeapingScale<-",
                ],
            ),
            (
                53,
                71,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary>+",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Auxiliary<-",
                    "Run>-",
                    "Run<-",
                    "Trill1>-",
                    "Trill1<-",
                    "ReturnCrazyDriver>-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                54,
                72,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary>+",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Auxiliary<-",
                    "Run>-",
                    "Run<-",
                    "Trill1>-",
                    "Trill1<-",
                    "ReturnCrazyDriver>-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                55,
                71,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Auxiliary<+",
                    "Auxiliary>-",
                    "Run>-",
                    "Run<-",
                    "Trill1>-",
                    "Trill1<-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                56,
                69,
                vec![
                    "Note3Scale>-",
                    "Note3Scale<-",
                    "Run>-",
                    "Run<-",
                    "ReturnCrazyDriver<-",
                ],
            ),
            (
                57,
                67,
                vec!["Note3Scale>-", "Note3Scale<-", "Run>-", "Run<-"],
            ),
        ];
        for ((i, figs), (ei, ep, efigs)) in figures.iter().zip(expected.iter()) {
            println!("{i}: {}", figures2string(figs.iter()));
            assert_eq!(i, ei);
            assert_eq!(melody[*i].pitch(), *ep);
            assert_eq!(figs.len(), efigs.len());
            for (fig, efig) in figs.iter().zip(efigs) {
                assert_eq!(format!("{fig}"), *efig);
            }
        }
        assert_eq!(figures.iter().count(), expected.len());

        let full_match = FigureMatcher::matching_figures(&melody);
        for ci in 0..figures.len() {
            let end = if ci + 1 == figures.len() {melody.len()} else {figures[ci + 1].0};
            for mi in figures[ci].0..end {
                assert_eq!(full_match[mi], figures[ci].1);
            }
        }
    }

    // Exploration tests. These do not contain assertions and I ultimately plan to
    // delete them, or evolve the into "real" tests. They are here to explore.

    #[test]
    fn test_note_projection() {
        let pitch = 60;
        let scale = ScaleMode::Major.rooted(NoteName::name_of(pitch));
        let mut patterns = BTreeSet::<Vec<u8>>::new();
        let mut option_sets_3 = vec![BTreeSet::new(), BTreeSet::new(), BTreeSet::new()];
        let mut option_sets_4 = vec![
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        ];
        for figure in all::<MelodicFigure>() {
            let projected = figure.projected_notes_from(pitch, &scale);
            patterns.insert(projected.clone());
            for i in 0..projected.len() {
                if figure.len() == 3 {
                    option_sets_3[i].insert(projected[i]);
                } else {
                    option_sets_4[i].insert(projected[i]);
                }
            }
            println!("{figure} {projected:?}");
        }
        println!("3-note figures");
        for i in 0..option_sets_3.len() {
            println!("options for {}: {:?}", i, option_sets_3[i]);
        }
        let combos3 = option_sets_3.iter().map(|n| n.len()).product::<usize>();
        println!("4-note figures");
        for i in 0..option_sets_4.len() {
            println!("options for {}: {:?}", i, option_sets_4[i]);
        }
        let combos4 = option_sets_4.iter().map(|n| n.len()).product::<usize>();
        println!("{} total projections", patterns.len());
        println!("Imaginable combos: 3: {combos3} 4: {combos4}");
    }

    #[test]
    fn show_tables() {
        let melody = Melody::from_file("joy_world_2")
            .unwrap()
            .without_ghosts(0.05);
        FigureMatcher::show_tables_for(&melody);
    }

    #[test]
    fn show_not_melodic() {
        let melody = Melody::from_file("NotMelodic2")
            .unwrap()
            .without_ghosts(0.05);
        let figures = FigureMatcher::matching_figures_consolidated(&melody);
        for (i, figs) in figures.iter() {
            let figstr = figs.iter().map(|f| format!("{f} ")).collect::<String>();
            println!("{i}: {} {figstr}\n", melody[*i].pitch());
        }

        FigureMatcher::show_tables_for(&melody);
    }
}
