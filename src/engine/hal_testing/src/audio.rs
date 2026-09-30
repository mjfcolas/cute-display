use std::sync::{Arc, Mutex};

use hal::audio::Speaker;
use hal::Fault;

use crate::shared::lock;

/// Keeps every sample as it is played, so a test can watch a sound while it plays.
#[derive(Clone, Default)]
pub struct StubSpeaker(Arc<Mutex<Recording>>);

#[derive(Default)]
struct Recording {
    samples: Vec<i16>,
    sounds: usize,
}

impl StubSpeaker {
    pub const SAMPLE_RATE_HZ: u32 = 8_000;

    /// Every sample played, one sound after the other.
    pub fn samples(&self) -> Vec<i16> {
        lock(&self.0).samples.clone()
    }

    pub fn samples_played(&self) -> usize {
        lock(&self.0).samples.len()
    }

    /// The sounds started, including one playing.
    pub fn sounds_played(&self) -> usize {
        lock(&self.0).sounds
    }
}

impl Speaker for StubSpeaker {
    fn sample_rate_hz(&self) -> u32 {
        Self::SAMPLE_RATE_HZ
    }

    fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
        lock(&self.0).sounds += 1;
        for sample in samples {
            lock(&self.0).samples.push(sample);
        }
        Ok(())
    }
}
