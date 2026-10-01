#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Volume(u8);

impl Volume {
    pub const fn percent(percent: u8) -> Self {
        Self(if percent > 100 { 100 } else { percent })
    }

    pub fn as_percent(self) -> u8 {
        self.0
    }
}

pub type Samples = Box<dyn Iterator<Item = i16> + Send>;

pub trait Sound: Send {
    fn sample_rate_hz(&self) -> u32;
    fn play(&mut self, samples: Samples);
    fn stop(&mut self);
}
