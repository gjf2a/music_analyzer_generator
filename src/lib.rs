use enum_iterator::Sequence;
use midi_note_recorder::note_velocity_from;
use midi_msg::MidiMsg;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Sequence)]
pub enum NoteLetter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl NoteLetter {
    pub fn next(&self) -> Self {
        enum_iterator::next_cycle(self)
    }

    pub fn prev(&self) -> Self {
        enum_iterator::previous_cycle(self)
    }

    pub fn natural_pitch(&self) -> u8 {
        match self {
            NoteLetter::C => 0,
            NoteLetter::D => 2,
            NoteLetter::E => 4,
            NoteLetter::F => 5,
            NoteLetter::G => 7,
            NoteLetter::A => 9,
            NoteLetter::B => 11,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Accidental {
    Flat,
    Natural,
    Sharp,
}

impl Accidental {
    pub fn symbol(&self) -> char {
        match self {
            Accidental::Flat => '\u{266d}',
            Accidental::Natural => '\u{266e}',
            Accidental::Sharp => '\u{266f}',
        }
    }

    pub fn pitch_shift(&self, natural: u8) -> Option<u8> {
        match self {
            Accidental::Flat => if natural > 0 {Some(natural - 1)} else {None},
            Accidental::Natural => Some(natural),
            Accidental::Sharp => if natural < u8::MAX {Some(natural + 1)} else {None},
        }
    }
}

pub enum ChordMode {
    Major, 
    Minor
}

impl ChordMode {
    
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct ActivePitches {
    on: u128
}

impl ActivePitches {
    pub fn update_from(&mut self, msg: &MidiMsg) {
        if let Some((pitch, velocity)) = note_velocity_from(msg) {
            if velocity > 0 {
                self.on |= 1 << pitch;
            } else {
                self.on &= !(1 << pitch);
            }
        }
    }

    pub fn active_pitches(&self) -> Vec<u8> {
        (0..=127).filter(|p| self.on & (1 << p) > 0).collect()
    }

    pub fn len(&self) -> usize {
        self.on.count_ones() as usize
    }

    pub fn is_active(&self, pitch: u8) -> bool {
        self.on & (1 << pitch) != 0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use midi_msg::Channel;
    use midi_note_recorder::midi_msg_from;
    use rand::Rng;

    use crate::ActivePitches;

    #[test]
    fn test_active_pitches() {
        let mut rng = rand::thread_rng();
        let mut active = ActivePitches::default();
        let mut active_tester = BTreeSet::new();
        for _ in 0..100 {
            if active.len() == 0 || rng.gen_bool(0.5) {
                let note = rng.gen_range(0..=127);
                let already = active.is_active(note);
                let msg = midi_msg_from(Channel::Ch1, note, 1);
                let prev_len = active.len();
                active.update_from(&msg);
                assert!(already && active.len() == prev_len || !already && active.len() == prev_len + 1);
                assert!(active.is_active(note));
                active_tester.insert(note);
            } else {
                let pitches = active.active_pitches();
                let remove = pitches[rng.gen_range(0..pitches.len())];
                let msg = midi_msg_from(Channel::Ch1, remove, 0);
                let prev_len = active.len();
                active.update_from(&msg);
                assert!(!active.is_active(remove));
                assert_eq!(prev_len - 1, active.len());
                active_tester.remove(&remove);
            }
            let comp = active_tester.iter().copied().collect::<Vec<_>>();
            assert_eq!(comp, active.active_pitches());
        }
    }
}