pub type Samples = Box<dyn Iterator<Item = i16> + Send>;

pub trait Sound: Send {
    fn sample_rate_hz(&self) -> u32;
    fn play(&mut self, samples: Samples);
    fn stop(&mut self);
}
