use enum_iterator::Sequence;

// Inspired by: https://figuringoutmelody.com/the-24-universal-melodic-figures/
#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence, Hash, Ord, PartialOrd)]
pub struct MelodicFigure {
    shape: MelodicFigureShape,
    polarity: FigurePolarity,
    direction: FigureDirection,
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