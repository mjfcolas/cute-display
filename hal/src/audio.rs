use crate::Fault;

pub trait Speaker {
    fn sample_rate_hz(&self) -> u32;

    /// Plays mono samples until the source runs out, powering the amplifier only for
    /// that long.
    fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault>;
}
