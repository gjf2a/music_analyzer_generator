use std::collections::{BTreeMap, HashSet};

use bare_metal_modulo::{MNum, ModNum};
use enum_iterator::{Sequence, all};
use hash_histogram::HashHistogram;

use crate::notes::{Accidental, NoteLetter, NoteName};

pub fn all_rooted_scales() -> impl Iterator<Item = RootedScale> {
    all::<ScaleMode>().flat_map(|mode| (60..72).map(move |p| mode.pitch_rooted(p)))
}

#[derive(Copy, Clone, Debug, Sequence, Eq, PartialEq)]
pub enum ScaleMode {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    WholeTone,
    HarmonicMinor,
    MelodicMinor,
    Diminished,
    Augmented,
}

impl ScaleMode {
    pub fn rooted(&self, root: NoteName) -> RootedScale {
        RootedScale::new(*self, root)
    }

    pub fn pitch_rooted(&self, root: u8) -> RootedScale {
        RootedScale::new(*self, NoteName::name_of(root))
    }

    pub fn is_symmetric(&self) -> bool {
        match self {
            Self::Augmented | Self::Diminished | Self::WholeTone => true,
            _ => false,
        }
    }

    fn scale_size(&self) -> usize {
        match self {
            Self::WholeTone | Self::Augmented => 6,
            Self::Diminished => 8,
            _ => 7,
        }
    }

    fn pattern_up(&self) -> ScalePattern {
        match self {
            Self::Major => ScalePattern::mode_rotation(0),
            Self::Dorian => ScalePattern::mode_rotation(1),
            Self::Phrygian => ScalePattern::mode_rotation(2),
            Self::Lydian => ScalePattern::mode_rotation(3),
            Self::Mixolydian => ScalePattern::mode_rotation(4),
            Self::Minor => ScalePattern::mode_rotation(5),
            Self::Locrian => ScalePattern::mode_rotation(6),
            Self::HarmonicMinor => ScalePattern::standard([2, 1, 2, 2, 1, 3, 1]),
            Self::MelodicMinor => ScalePattern::standard([2, 1, 2, 2, 2, 2, 1]),
            Self::WholeTone => ScalePattern {
                num_jumps: 6,
                jumps: [2, 2, 2, 2, 2, 2, 0, 0],
            },
            Self::Diminished => ScalePattern {
                num_jumps: 8,
                jumps: [2, 1, 2, 1, 2, 1, 2, 1],
            },
            Self::Augmented => ScalePattern {
                num_jumps: 6,
                jumps: [3, 1, 3, 1, 3, 1, 0, 0],
            },
        }
    }

    pub fn letter_iterator(&self, root_letter: NoteLetter) -> ScaleLetterIterator {
        let mut letter_seq = all::<NoteLetter>()
            .cycle()
            .skip_while(|nl| *nl != root_letter)
            .take(7)
            .collect::<Vec<_>>();
        match self {
            Self::WholeTone => {
                letter_seq.pop();
            }
            Self::Augmented => {
                letter_seq.remove(5);
                letter_seq[3] = letter_seq[4];
            }
            Self::Diminished => {
                letter_seq.insert(4, letter_seq[4]);
            }
            _ => {}
        }
        ScaleLetterIterator {
            pos: ModNum::new(0, letter_seq.len()),
            letter_seq,
        }
    }

    fn notes_going_up(&self, note: NoteName) -> impl Iterator<Item = u8> {
        ScaleUpIterator {
            pattern: self.pattern_up(),
            current: note.lowest_midi_note(),
            count: 0,
        }
    }

    fn notes_going_down(&self, note: NoteName) -> impl Iterator<Item = u8> {
        let root_note = note.lowest_midi_note();
        ScaleDownIterator {
            pattern: self.pattern_down(),
            current: root_note + if root_note > 7 { 108 } else { 120 },
            count: 0,
        }
    }

    fn pattern_down(&self) -> ScalePattern {
        match self {
            Self::MelodicMinor => ScalePattern::mode_rotation(5),
            _ => self.pattern_up(),
        }
        .reversed()
    }

    pub fn characteristic_notes(&self) -> Vec<usize> {
        // Took inspiration from this discussion: https://www.reddit.com/r/musictheory/comments/nf2qir/what_are_the_characteristic_notes_of_the_modes/
        match self {
            Self::Major | Self::Lydian | Self::Mixolydian => vec![1, 3, 4, 7],
            Self::Minor | Self::Dorian | Self::Phrygian => vec![1, 2, 3, 6],
            Self::MelodicMinor | Self::HarmonicMinor => vec![1, 3, 6, 7],
            Self::Locrian => vec![1, 2, 5, 6],
            Self::WholeTone | Self::Augmented => vec![1, 2, 3, 4, 5, 6],
            Self::Diminished => vec![1, 2, 3, 4, 5, 6, 7, 8],
        }
    }
}

pub struct ScaleLetterIterator {
    pos: ModNum<usize>,
    letter_seq: Vec<NoteLetter>,
}

impl Iterator for ScaleLetterIterator {
    type Item = NoteLetter;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.letter_seq[self.pos.a()];
        self.pos += 1;
        Some(result)
    }
}

impl DoubleEndedIterator for ScaleLetterIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        let result = self.letter_seq[self.pos.a()];
        self.pos -= 1;
        Some(result)
    }
}

#[derive(Debug, Clone)]
pub struct RootedScale {
    mode: ScaleMode,
    root: NoteName,
    note_weights_up: HashHistogram<NoteName, f64>,
    note_weights_down: HashHistogram<NoteName, f64>,
    notes2letters_up: BTreeMap<u8, NoteLetter>,
    notes2names_up: BTreeMap<u8, NoteName>,
    notes2letters_down: BTreeMap<u8, NoteLetter>,
    notes2names_down: BTreeMap<u8, NoteName>,
    all_note_names: HashSet<NoteName>,
}

impl RootedScale {
    pub fn new(mode: ScaleMode, root: NoteName) -> Self {
        let mut result = Self {
            mode,
            root,
            note_weights_up: HashHistogram::new(),
            note_weights_down: HashHistogram::new(),
            notes2letters_up: BTreeMap::default(),
            notes2names_up: BTreeMap::default(),
            notes2letters_down: BTreeMap::default(),
            notes2names_down: BTreeMap::default(),
            all_note_names: HashSet::default(),
        };
        let notes2letters_up = result.all_diatonic_note_letters_up().collect();
        result.notes2letters_up = notes2letters_up;
        let notes2names = result.all_diatonic_notes_up().collect();
        result.notes2names_up = notes2names;
        let notes2letters_down = result.all_diatonic_note_letters_down().collect();
        result.notes2letters_down = notes2letters_down;
        let notes2names_down = result.all_diatonic_notes_down().collect();
        result.notes2names_down = notes2names_down;
        for note_name in result.notes2names_up.values() {
            result.all_note_names.insert(*note_name);
        }
        for note_name in result.notes2names_down.values() {
            result.all_note_names.insert(*note_name);
        }
        let scale_size = mode.scale_size();
        let characteristic_tones: Vec<usize> = mode.characteristic_notes();
        for (i, (_, note_name)) in result.notes2names_up.iter().take(scale_size).enumerate() {
            let weight = if i == 0 {
                4.0
            } else if characteristic_tones.contains(&(i + 1)) {
                2.0
            } else {
                1.0
            };
            result.note_weights_up.bump_by(note_name, weight);
        }
        result.note_weights_up.normalize(1.0);

        for (i, (_, note_name)) in result.notes2names_down.iter().take(scale_size).enumerate() {
            let tone_index = if i == 0 { 1 } else { scale_size + 2 - i };
            let weight = if characteristic_tones.contains(&tone_index) {
                2.0
            } else {
                1.0
            };
            result.note_weights_down.bump_by(note_name, weight);
        }
        result.note_weights_down.normalize(1.0);
        result
    }

    pub fn ascending_note_weight(&self, pitch: u8) -> Option<(NoteName, f64)> {
        self.name_of_ascending(pitch)
            .map(|name| (name, self.note_weights_up.count(&name)))
    }

    pub fn descending_note_weight(&self, pitch: u8) -> Option<(NoteName, f64)> {
        self.name_of(pitch)
            .map(|name| (name, self.note_weights_down.count(&name)))
    }

    pub fn root_name(&self) -> NoteName {
        self.root
    }

    pub fn mode(&self) -> ScaleMode {
        self.mode
    }

    pub fn all_diatonic_note_letters_up(&self) -> impl Iterator<Item = (u8, NoteLetter)> {
        self.notes_going_up()
            .zip(self.mode.letter_iterator(self.root.letter()))
    }

    pub fn all_diatonic_notes_up(&self) -> impl Iterator<Item = (u8, NoteName)> {
        self.all_diatonic_note_letters_up()
            .map(|(pitch, letter)| (pitch, NoteName::full_name_for(letter, pitch)))
    }

    pub fn all_diatonic_note_letters_down(&self) -> impl Iterator<Item = (u8, NoteLetter)> {
        self.notes_going_down()
            .zip(self.mode.letter_iterator(self.root.letter()).rev())
    }

    pub fn all_diatonic_notes_down(&self) -> impl Iterator<Item = (u8, NoteName)> {
        self.all_diatonic_note_letters_down()
            .map(|(pitch, letter)| (pitch, NoteName::full_name_for(letter, pitch)))
    }

    pub fn all_diatonic_note_names_unordered(&self) -> impl Iterator<Item = NoteName> {
        self.all_note_names.iter().copied()
    }

    pub fn contains(&self, note: u8) -> bool {
        self.notes2letters_down.contains_key(&note)
    }

    pub fn contains_ascending(&self, note: u8) -> bool {
        self.notes2letters_up.contains_key(&note)
    }

    pub fn name_of(&self, pitch: u8) -> Option<NoteName> {
        self.notes2names_down.get(&pitch).copied()
    }

    pub fn name_of_ascending(&self, pitch: u8) -> Option<NoteName> {
        self.notes2names_up.get(&pitch).copied()
    }

    pub fn middle_c(&self) -> u8 {
        for candidate in [60, 61, 59, 62, 58] {
            if self.contains(candidate) {
                return candidate;
            }
        }
        panic!("None of the C-adjacent notes are present in this scale.");
    }

    pub fn diatonic_steps_to_middle_c(&self, pitch: u8) -> Option<u8> {
        let c_major = ScaleMode::Major.pitch_rooted(60);
        let mode = if self.mode.is_symmetric() {
            &c_major
        } else {
            self
        };
        mode.diatonic_steps_between(mode.middle_c(), mode.round_up(pitch))
    }

    pub fn all_sharps(&self) -> impl Iterator<Item = NoteLetter> {
        self.all_diatonic_notes_down()
            .take(7)
            .filter(|(_, n)| !self.mode.is_symmetric() && n.accidental() == Accidental::Sharp)
            .map(|(_, n)| n.letter())
    }

    pub fn all_flats(&self) -> impl Iterator<Item = NoteLetter> {
        self.all_diatonic_notes_down()
            .take(7)
            .filter(|(_, n)| !self.mode.is_symmetric() && n.accidental() == Accidental::Flat)
            .map(|(_, n)| n.letter())
    }

    pub fn round_up(&self, pitch: u8) -> u8 {
        assert!(pitch <= *self.notes2letters_down.last_key_value().unwrap().0);
        let mut rounded = pitch;
        while !self.contains(rounded) {
            rounded += 1;
        }
        rounded
    }

    pub fn round_down(&self, pitch: u8) -> u8 {
        let mut rounded = pitch;
        while !self.contains(rounded) {
            rounded -= 1;
        }
        rounded
    }

    pub fn notes_going_up(&self) -> impl Iterator<Item = u8> {
        self.mode.notes_going_up(self.root)
    }

    pub fn notes_going_down(&self) -> impl Iterator<Item = u8> {
        self.mode.notes_going_down(self.root)
    }

    pub fn note_up(&self, current: u8, interval: usize) -> Option<u8> {
        self.notes_going_up()
            .skip_while(|n| *n < current)
            .skip(interval - 1)
            .next()
    }

    pub fn note_down(&self, current: u8, interval: usize) -> Option<u8> {
        self.notes_going_down()
            .skip_while(|n| *n > current)
            .skip(interval - 1)
            .next()
    }

    pub fn diatonic_bracket_for(&self, pitch: u8) -> Option<(u8, u8)> {
        if self.contains(pitch) {
            None
        } else {
            let below = self.round_down(pitch);
            let above = self.round_up(pitch);
            Some((below, above))
        }
    }

    pub fn diatonic_steps_between_up(&self, pitch_1: u8, pitch_2: u8) -> Option<u8> {
        if pitch_1 > pitch_2 {
            self.diatonic_steps_between_up(pitch_2, pitch_1)
        } else {
            let interval = self
                .notes_going_up()
                .skip_while(|n| *n < pitch_1)
                .take_while(|n| *n <= pitch_2)
                .collect::<Vec<_>>();
            if interval.len() == 0
                || interval[0] != pitch_1
                || interval[interval.len() - 1] != pitch_2
            {
                None
            } else {
                Some((interval.len() - 1) as u8)
            }
        }
    }

    pub fn diatonic_steps_between(&self, lo_pitch: u8, hi_pitch: u8) -> Option<u8> {
        self.notes_between_down(lo_pitch, hi_pitch)
            .or(self.notes_between_up(lo_pitch, hi_pitch))
            .map(|interval| (interval.len() - 1) as u8)
    }

    pub fn notes_between_down(&self, lo_pitch: u8, hi_pitch: u8) -> Option<Vec<u8>> {
        if lo_pitch > hi_pitch {
            self.notes_between_down(hi_pitch, lo_pitch)
        } else {
            let interval = self
                .notes_going_down()
                .skip_while(|n| *n > hi_pitch)
                .take_while(|n| *n >= lo_pitch)
                .collect::<Vec<_>>();
            if interval.len() == 0
                || interval[0] != hi_pitch
                || interval[interval.len() - 1] != lo_pitch
            {
                None
            } else {
                Some(interval)
            }
        }
    }

    pub fn notes_between_up(&self, lo_pitch: u8, hi_pitch: u8) -> Option<Vec<u8>> {
        if lo_pitch > hi_pitch {
            self.notes_between_up(hi_pitch, lo_pitch)
        } else {
            let interval = self
                .notes_going_up()
                .skip_while(|n| *n < lo_pitch)
                .take_while(|n| *n <= hi_pitch)
                .collect::<Vec<_>>();
            if interval.len() == 0
                || interval[0] != lo_pitch
                || interval[interval.len() - 1] != hi_pitch
            {
                None
            } else {
                Some(interval)
            }
        }
    }

    pub fn descending_match(&self, pitch: u8) -> (NoteName, u8, Option<Accidental>) {
        if let Some((down, up)) = self.diatonic_bracket_for(pitch) {
            let down_name = self.name_of(down).unwrap();
            let up_name = self.name_of(up).unwrap();
            let down_modifier = down_name.accidental().sharpen();
            let up_modifier = up_name.accidental().flatten();
            if let Some(up_modifier) = up_modifier {
                if up_modifier.offset_value() > -2 {
                    return (up_name, up, Some(up_modifier));
                }
            }
            (down_name, down, down_modifier)
        } else {
            (self.name_of(pitch).unwrap(), pitch, None)
        }
    }

    pub fn ascending_match(&self, pitch: u8) -> (NoteName, u8, Option<Accidental>) {
        if let Some((down, up)) = self.diatonic_bracket_for(pitch) {
            let down_name = self.name_of(down).unwrap();
            let up_name = self.name_of(up).unwrap();
            let down_modifier = down_name.accidental().sharpen();
            let up_modifier = up_name.accidental().flatten();
            if let Some(down_modifier) = down_modifier {
                if down_modifier.offset_value() < 2 {
                    return (down_name, down, Some(down_modifier));
                }
            }
            (up_name, up, up_modifier)
        } else {
            (self.name_of(pitch).unwrap(), pitch, None)
        }
    }
}

struct ScaleUpIterator {
    pattern: ScalePattern,
    current: u8,
    count: usize,
}

impl Iterator for ScaleUpIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= 127 {
            let result = Some(self.current);
            self.current += self.pattern.jump(self.count);
            self.count += 1;
            result
        } else {
            None
        }
    }
}

struct ScaleDownIterator {
    pattern: ScalePattern,
    current: u8,
    count: usize,
}

impl Iterator for ScaleDownIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.pattern.jump(self.count) {
            let result = Some(self.current);
            self.current -= self.pattern.jump(self.count);
            self.count += 1;
            result
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct ScalePattern {
    num_jumps: usize,
    jumps: [u8; 8],
}

impl ScalePattern {
    fn jump(&self, count: usize) -> u8 {
        self.jumps[count % self.num_jumps]
    }

    fn reversed(&self) -> Self {
        let mut result = *self;
        for i in 0..self.num_jumps {
            result.jumps[i] = self.jumps[self.num_jumps - i - 1];
        }
        result
    }

    fn standard(intervals: [u8; 7]) -> Self {
        let mut jumps = [0; 8];
        for (i, j) in intervals.iter().enumerate() {
            jumps[i] = *j;
        }
        Self {
            num_jumps: 7,
            jumps,
        }
    }

    fn mode_rotation(rotation: usize) -> Self {
        let major = [2, 2, 1, 2, 2, 2, 1];
        let mut destination = [0; 7];
        for i in 0..major.len() {
            destination[i] = major[(i + rotation) % major.len()];
        }
        Self::standard(destination)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::{MAJOR_ROOT_IDS, MINOR_ROOT_IDS, NoteName};

    use crate::notes::Accidental::Flat as F;
    use crate::notes::Accidental::Natural as N;
    use crate::notes::Accidental::Sharp as S;
    use crate::notes::NoteLetter as NL;
    use crate::scales::ScaleMode as SM;

    #[test]
    fn test_ascending_scale() {
        let scale = SM::Major.rooted(NoteName::new(NL::C, N));
        let c_notes = scale.notes_going_up().collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24]
        );
    }

    #[test]
    fn test_descending_scale() {
        let c_notes = SM::Major
            .rooted(NoteName::new(NL::C, N))
            .notes_going_down()
            .collect::<Vec<_>>();
        assert_eq!(
            c_notes[..15],
            vec![
                120, 119, 117, 115, 113, 112, 110, 108, 107, 105, 103, 101, 100, 98, 96
            ]
        );
    }

    #[test]
    fn test_note_up() {
        let root1 = NoteName::new(NL::C, N);
        let root2 = NoteName::new(NL::F, S);
        let root3 = NoteName::new(NL::B, F);
        for (root, mode, current, interval, expected) in [
            (root1, SM::Major, 60, 3, 64),
            (root1, SM::Minor, 60, 3, 63),
            (root1, SM::Phrygian, 60, 2, 61),
            (root2, SM::MelodicMinor, 66, 1, 66),
            (root2, SM::MelodicMinor, 66, 6, 75),
            (root3, SM::MelodicMinor, 58, 7, 69),
        ] {
            let scale = mode.rooted(root);
            assert_eq!(scale.note_up(current, interval).unwrap(), expected);
        }
    }

    #[test]
    fn test_note_down() {
        let root1 = NoteName::new(NL::C, N);
        let root2 = NoteName::new(NL::F, S);
        let root3 = NoteName::new(NL::B, F);
        for (root, mode, current, interval, expected) in [
            (root1, SM::Major, 60, 3, 57),
            (root1, SM::Minor, 60, 3, 56),
            (root1, SM::Phrygian, 60, 2, 58),
            (root2, SM::MelodicMinor, 66, 2, 64),
            (root2, SM::MelodicMinor, 66, 3, 62),
            (root3, SM::MelodicMinor, 58, 7, 48),
        ] {
            let scale = mode.rooted(root);
            assert_eq!(scale.note_down(current, interval).unwrap(), expected);
        }
    }

    #[test]
    fn test_middle_c() {
        let expected = [60, 60, 61, 60, 61, 60, 61, 60, 60, 61, 60, 61];
        for i in 0..expected.len() {
            let note = NoteName::new(MAJOR_ROOT_IDS[i].0, MAJOR_ROOT_IDS[i].1);
            assert_eq!(expected[i], SM::Major.rooted(note).middle_c());
        }
    }

    #[test]
    fn test_diatonic_intervals() {
        for (root, mode, p1, p2, expected) in [
            (71, SM::Major, 70, 75, Some(3)),
            (71, SM::Major, 75, 70, Some(3)),
            (71, SM::Major, 70, 74, None),
            (67, SM::Major, 71, 71, Some(0)),
            (62, SM::Dorian, 65, 74, Some(5)),
            (71, SM::Augmented, 60, 79, None),
            (71, SM::Augmented, 61, 79, None),
            (71, SM::Augmented, 59, 79, Some(10)),
            (60, SM::Major, 41, 79, Some(22)),
            (69, SM::MelodicMinor, 41, 79, Some(22)),
        ] {
            let scale = mode.pitch_rooted(root);
            assert_eq!(scale.diatonic_steps_between(p1, p2), expected);
        }
    }

    #[test]
    fn test_round_up() {
        for (root, mode, pitch, expected) in [
            (65, SM::Major, 71, 72),
            (65, SM::Major, 72, 72),
            (71, SM::Augmented, 79, 79),
        ] {
            let scale = mode.pitch_rooted(root);
            assert_eq!(scale.round_up(pitch), expected);
        }
    }

    #[test]
    fn test_round_down() {
        for (root, mode, pitch, expected) in [(65, SM::Major, 71, 70), (65, SM::Major, 72, 72)] {
            let scale = mode.pitch_rooted(root);
            assert_eq!(scale.round_down(pitch), expected);
        }
    }

    #[test]
    fn test_note_letters() {
        for (scale, root, letters) in [
            (
                SM::Major,
                60,
                [
                    (0, NL::C),
                    (2, NL::D),
                    (4, NL::E),
                    (5, NL::F),
                    (7, NL::G),
                    (9, NL::A),
                    (11, NL::B),
                    (12, NL::C),
                    (14, NL::D),
                    (16, NL::E),
                    (17, NL::F),
                    (19, NL::G),
                    (21, NL::A),
                    (23, NL::B),
                    (24, NL::C),
                ],
            ),
            (
                SM::Major,
                59,
                [
                    (11, NL::B),
                    (13, NL::C),
                    (15, NL::D),
                    (16, NL::E),
                    (18, NL::F),
                    (20, NL::G),
                    (22, NL::A),
                    (23, NL::B),
                    (25, NL::C),
                    (27, NL::D),
                    (28, NL::E),
                    (30, NL::F),
                    (32, NL::G),
                    (34, NL::A),
                    (35, NL::B),
                ],
            ),
            (
                SM::Minor,
                58,
                [
                    (10, NL::B),
                    (12, NL::C),
                    (13, NL::D),
                    (15, NL::E),
                    (17, NL::F),
                    (18, NL::G),
                    (20, NL::A),
                    (22, NL::B),
                    (24, NL::C),
                    (25, NL::D),
                    (27, NL::E),
                    (29, NL::F),
                    (30, NL::G),
                    (32, NL::A),
                    (34, NL::B),
                ],
            ),
            (
                SM::Minor,
                60,
                [
                    (0, NL::C),
                    (2, NL::D),
                    (3, NL::E),
                    (5, NL::F),
                    (7, NL::G),
                    (8, NL::A),
                    (10, NL::B),
                    (12, NL::C),
                    (14, NL::D),
                    (15, NL::E),
                    (17, NL::F),
                    (19, NL::G),
                    (20, NL::A),
                    (22, NL::B),
                    (24, NL::C),
                ],
            ),
        ] {
            let rooted = scale.pitch_rooted(root);
            let values = rooted
                .all_diatonic_note_letters_up()
                .take(letters.len())
                .collect::<Vec<_>>();
            for i in 0..letters.len() {
                assert_eq!(values[i], letters[i]);
            }
        }
    }

    #[test]
    fn test_note_name_letters() {
        for (scale, root, letters) in [
            (
                SM::Major,
                60,
                [
                    (0, NL::C, N),
                    (2, NL::D, N),
                    (4, NL::E, N),
                    (5, NL::F, N),
                    (7, NL::G, N),
                    (9, NL::A, N),
                    (11, NL::B, N),
                    (12, NL::C, N),
                    (14, NL::D, N),
                    (16, NL::E, N),
                    (17, NL::F, N),
                    (19, NL::G, N),
                    (21, NL::A, N),
                    (23, NL::B, N),
                    (24, NL::C, N),
                ],
            ),
            (
                SM::Major,
                59,
                [
                    (11, NL::B, N),
                    (13, NL::C, S),
                    (15, NL::D, S),
                    (16, NL::E, N),
                    (18, NL::F, S),
                    (20, NL::G, S),
                    (22, NL::A, S),
                    (23, NL::B, N),
                    (25, NL::C, S),
                    (27, NL::D, S),
                    (28, NL::E, N),
                    (30, NL::F, S),
                    (32, NL::G, S),
                    (34, NL::A, S),
                    (35, NL::B, N),
                ],
            ),
            (
                SM::Minor,
                58,
                [
                    (10, NL::B, F),
                    (12, NL::C, N),
                    (13, NL::D, F),
                    (15, NL::E, F),
                    (17, NL::F, N),
                    (18, NL::G, F),
                    (20, NL::A, F),
                    (22, NL::B, F),
                    (24, NL::C, N),
                    (25, NL::D, F),
                    (27, NL::E, F),
                    (29, NL::F, N),
                    (30, NL::G, F),
                    (32, NL::A, F),
                    (34, NL::B, F),
                ],
            ),
            (
                SM::Minor,
                60,
                [
                    (0, NL::C, N),
                    (2, NL::D, N),
                    (3, NL::E, F),
                    (5, NL::F, N),
                    (7, NL::G, N),
                    (8, NL::A, F),
                    (10, NL::B, F),
                    (12, NL::C, N),
                    (14, NL::D, N),
                    (15, NL::E, F),
                    (17, NL::F, N),
                    (19, NL::G, N),
                    (20, NL::A, F),
                    (22, NL::B, F),
                    (24, NL::C, N),
                ],
            ),
        ] {
            let rooted = scale.pitch_rooted(root);
            let values = rooted
                .all_diatonic_notes_up()
                .take(letters.len())
                .collect::<Vec<_>>();
            for i in 0..letters.len() {
                let (pitch, letter, modifier) = letters[i];
                let name = NoteName::new(letter, modifier);
                assert_eq!(values[i], (pitch, name));
            }
        }
    }

    #[test]
    fn test_all_flats() {
        for (scale, root, target) in [
            (SM::Major, 60, vec![]),
            (SM::Minor, 60, vec![NL::B, NL::A, NL::E]),
            (SM::Major, 62, vec![]),
            (SM::Major, 17, vec![NL::B]),
            (SM::Minor, 17, vec![NL::E, NL::D, NL::B, NL::A]),
            (SM::Major, 61, vec![NL::D, NL::B, NL::A, NL::G, NL::E]),
        ] {
            let rooted = scale.pitch_rooted(root);
            assert_eq!(target, rooted.all_flats().collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_all_sharps() {
        for (scale, root, target) in [
            (SM::Major, 60, vec![]),
            (SM::Minor, 60, vec![]),
            (SM::Major, 62, vec![NL::C, NL::F]),
            (SM::Major, 59, vec![NL::A, NL::G, NL::F, NL::D, NL::C]),
            (
                SM::Major,
                18,
                vec![NL::F, NL::E, NL::D, NL::C, NL::A, NL::G],
            ),
        ] {
            let rooted = scale.pitch_rooted(root);
            assert_eq!(target, rooted.all_sharps().collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_diatonic_bracket() {
        for (scale, root, note, expected) in [
            (SM::Major, 60, 61, Some((60, 62))),
            (SM::Minor, 69, 61, Some((60, 62))),
            (SM::Major, 67, 73, Some((72, 74))),
            (SM::Major, 67, 72, None),
            (SM::Major, 59, 67, Some((66, 68))),
            (SM::Augmented, 60, 65, Some((64, 67))),
            (SM::Augmented, 60, 66, Some((64, 67))),
        ] {
            let rooted = scale.pitch_rooted(root);
            assert_eq!(expected, rooted.diatonic_bracket_for(note));
        }
    }

    #[test]
    fn test_mode_iterator() {
        for (scale, letter, expected) in [
            (
                SM::Major,
                NL::D,
                vec![
                    NL::D,
                    NL::E,
                    NL::F,
                    NL::G,
                    NL::A,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
                ],
            ),
            (
                SM::Minor,
                NL::A,
                vec![
                    NL::A,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
                    NL::F,
                    NL::G,
                    NL::A,
                    NL::B,
                ],
            ),
            (
                SM::Dorian,
                NL::F,
                vec![
                    NL::F,
                    NL::G,
                    NL::A,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
                    NL::F,
                    NL::G,
                ],
            ),
            (
                SM::Augmented,
                NL::C,
                vec![
                    NL::C,
                    NL::D,
                    NL::E,
                    NL::G,
                    NL::G,
                    NL::B,
                    NL::C,
                    NL::D,
                    NL::E,
                ],
            ),
        ] {
            let letters = scale.letter_iterator(letter).take(9).collect::<Vec<_>>();
            assert_eq!(expected, letters);
        }
    }

    #[test]
    fn test_melodic_minor() {
        for (letter, modifier) in MINOR_ROOT_IDS.iter().copied() {
            println!("New loop: {letter:?} {modifier:?}");
            let root = NoteName::new(letter, modifier);
            let scale = SM::MelodicMinor.rooted(root);
            let mut ups = scale.all_diatonic_notes_up().collect::<VecDeque<_>>();
            let mut dns = scale.all_diatonic_notes_down().collect::<VecDeque<_>>();
            while ups[ups.len() - 1] != dns[0] {
                ups.pop_back();
            }
            while ups[0] != dns[dns.len() - 1] && ups[0].0 < dns[dns.len() - 1].0 {
                println!("{:?} {:?} {}", ups[0], dns[dns.len() - 1], ups.len());
                ups.pop_front();
            }
            while ups[0] != dns[dns.len() - 1] && ups[0].0 > dns[dns.len() - 1].0 {
                dns.pop_back();
            }
            while ups[0].1.letter() != letter {
                ups.pop_front();
                dns.pop_back();
            }
            while dns[0].1.letter() != letter {
                ups.pop_back();
                dns.pop_front();
            }
            assert_eq!(ups.len(), dns.len());
            for ui in 0..ups.len() {
                let di = dns.len() - ui - 1;
                assert_eq!(ups[ui].1.letter(), dns[di].1.letter());
                if ui % 7 < 5 {
                    assert_eq!(ups[ui], dns[di]);
                } else {
                    assert_eq!(ups[ui].0, dns[di].0 + 1);
                    assert_eq!(
                        ups[ui].1.accidental().offset_value(),
                        dns[di].1.accidental().offset_value() + 1
                    );
                }
            }
        }
    }

    #[test]
    fn test_ascending_match() {
        for (
            scale,
            root,
            pitch,
            expected_letter,
            expected_modifier,
            expected_pitch,
            expected_ascend,
        ) in [
            (SM::Major, 60, 72, NL::C, N, 72, None),
            (SM::Major, 60, 73, NL::C, N, 72, Some(S)),
            (SM::Major, 59, 65, NL::E, N, 64, Some(S)),
            (SM::Major, 59, 67, NL::G, S, 68, Some(N)),
        ] {
            let rooted = scale.pitch_rooted(root);
            let (name, diatonic_pitch, ascend) = rooted.ascending_match(pitch);
            assert_eq!(expected_letter, name.letter());
            assert_eq!(expected_modifier, name.accidental());
            assert_eq!(expected_pitch, diatonic_pitch);
            assert_eq!(expected_ascend, ascend);
        }
    }

    #[test]
    fn test_descending_match() {
        for (
            scale,
            root,
            pitch,
            expected_letter,
            expected_modifier,
            expected_pitch,
            expected_descend,
        ) in [
            (SM::Major, 60, 72, NL::C, N, 72, None),
            (SM::Major, 60, 73, NL::D, N, 74, Some(F)),
            (SM::Major, 61, 71, NL::C, N, 72, Some(F)),
            (SM::Major, 61, 69, NL::A, F, 68, Some(N)),
        ] {
            let rooted = scale.pitch_rooted(root);
            let (name, diatonic_pitch, descend) = rooted.descending_match(pitch);
            assert_eq!(expected_letter, name.letter());
            assert_eq!(expected_modifier, name.accidental());
            assert_eq!(expected_pitch, diatonic_pitch);
            assert_eq!(expected_descend, descend);
        }
    }

    #[test]
    fn test_steps_to_middle_c() {
        for (ltr, acc, mode, test_pitch, target) in
            [(NL::C, N, SM::Major, 79, 11), (NL::B, N, SM::Augmented, 79, 11), (NL::A, N, SM::MelodicMinor, 79, 11)]
        {
            let scale = mode.rooted(NoteName::new(ltr, acc));
            assert_eq!(scale.diatonic_steps_to_middle_c(test_pitch), Some(target));
        }
    }
}
