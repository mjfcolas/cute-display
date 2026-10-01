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

/// Hears the first second of each sound and moves on, so that a sound that lasts until it
/// is stopped, a ringing alarm, does not keep its thread busy.
#[derive(Clone, Default)]
pub struct StubBriefSpeaker(Arc<Mutex<Heard>>);

#[derive(Default)]
struct Heard {
    sounds: usize,
    audible: usize,
}

impl StubBriefSpeaker {
    pub const SAMPLE_RATE_HZ: u32 = 8_000;

    pub fn sounds_started(&self) -> usize {
        lock(&self.0).sounds
    }

    /// The sounds started whose first second was not silence.
    pub fn sounds_heard(&self) -> usize {
        lock(&self.0).audible
    }
}

impl Speaker for StubBriefSpeaker {
    fn sample_rate_hz(&self) -> u32 {
        Self::SAMPLE_RATE_HZ
    }

    fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
        let audible = samples.take(Self::SAMPLE_RATE_HZ as usize).any(|sample| sample != 0);
        let mut heard = lock(&self.0);
        heard.sounds += 1;
        heard.audible += usize::from(audible);
        Ok(())
    }
}
