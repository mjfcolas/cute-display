use core::f32::consts::TAU;
use core::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

use hal::audio::Speaker;
use hal::Fault;

use crate::latest::Latest;

/// C5 E5 G5 C6.
const NOTES: [(f32, u32); 4] = [(523.25, 140), (659.25, 140), (783.99, 140), (1046.5, 320)];
/// Of full scale. The amplifier is loud: a pure tone at a quarter of full scale hurts.
const AMPLITUDE: f32 = 0.08;
const FADE_MS: u32 = 10;

/// A rising arpeggio, faded in and out note by note so nothing clicks.
pub struct Chime {
    sample_rate_hz: u32,
    note: usize,
    sample: u32,
}

impl Chime {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self { sample_rate_hz, note: 0, sample: 0 }
    }

    fn samples_in(&self, ms: u32) -> u32 {
        self.sample_rate_hz * ms / 1000
    }
}

impl Iterator for Chime {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let &(hz, ms) = NOTES.get(self.note)?;
        let length = self.samples_in(ms);
        if self.sample >= length {
            self.note += 1;
            self.sample = 0;
            return self.next();
        }
        let n = self.sample;
        self.sample += 1;
        let fade = self.samples_in(FADE_MS).max(1);
        let envelope = n.min(length - 1 - n).min(fade) as f32 / fade as f32;
        let wave = (TAU * hz * n as f32 / self.sample_rate_hz as f32).sin();
        Some((wave * envelope * AMPLITUDE * f32::from(i16::MAX)) as i16)
    }
}

/// Rings chimes on a thread of its own, since playing blocks until the sound ends.
#[derive(Clone)]
pub struct Chimes {
    requests: Sender<()>,
    played: Arc<AtomicU32>,
    failure: Latest<Option<Fault>>,
}

impl Chimes {
    pub fn spawn(mut speaker: impl Speaker + Send + 'static) -> Result<Self, Fault> {
        let (requests, pending) = mpsc::channel();
        let chimes = Self { requests, played: Arc::default(), failure: Latest::new(None) };
        let (played, failure) = (Arc::clone(&chimes.played), chimes.failure.clone());
        thread::Builder::new()
            .name("chimes".into())
            .stack_size(8 * 1024)
            .spawn(move || {
                for () in pending {
                    match speaker.play(&mut Chime::new(speaker.sample_rate_hz())) {
                        Ok(()) => {
                            played.fetch_add(1, Ordering::Relaxed);
                            failure.set(None);
                        }
                        Err(fault) => failure.set(Some(fault)),
                    }
                }
            })
            .map_err(Fault::new)?;
        Ok(chimes)
    }

    pub fn ring(&self) {
        if self.requests.send(()).is_err() {
            self.failure.set(Some(Fault::new("the chime thread is gone")));
        }
    }

    pub fn played(&self) -> u32 {
        self.played.load(Ordering::Relaxed)
    }

    pub fn failure(&self) -> Option<Fault> {
        self.failure.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chime_lasts_as_long_as_its_notes() {
        let total_ms: u32 = NOTES.iter().map(|&(_, ms)| ms).sum();
        assert_eq!(Chime::new(44_100).count() as u32, 44_100 * total_ms / 1000);
    }

    #[test]
    fn a_chime_stays_quiet_and_starts_and_ends_on_silence() {
        let samples: Vec<i16> = Chime::new(44_100).collect();
        let ceiling = (AMPLITUDE * f32::from(i16::MAX)) as i16 + 1;
        assert!(samples.iter().all(|s| s.abs() <= ceiling));
        assert_eq!(samples.first(), Some(&0));
        assert!(samples.last().is_some_and(|s| s.abs() < 50));
    }
}
