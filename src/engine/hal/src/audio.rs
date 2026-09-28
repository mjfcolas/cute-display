use crate::Fault;

pub trait Speaker {
    fn sample_rate_hz(&self) -> u32;

    fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault>;
}
