//! What apps play on the speaker, one sound at a time: a new one cuts short the one
//! playing.

/// Mono samples at the sound's sample rate; full scale is as loud as the device gets.
pub type Samples = Box<dyn Iterator<Item = i16> + Send>;

pub trait Sound: Send {
    fn sample_rate_hz(&self) -> u32;
    /// Plays `samples` until they run out or are stopped; returns at once.
    fn play(&mut self, samples: Samples);
    /// Cuts short what is playing.
    fn stop(&mut self);
}
