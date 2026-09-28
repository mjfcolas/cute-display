use core::f32::consts::TAU;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use domain::sound::Sound;

use crate::domain::alarm_clock::{Ringer, Volume};

/// C5 E5 G5 C6, then silence; in milliseconds.
const PHRASE: [(Option<f32>, u32); 5] = [(Some(523.25), 150), (Some(659.25), 150), (Some(783.99), 150), (Some(1046.5), 300), (None, 700)];
const FADE_MS: u32 = 10;

pub struct SoundRinger {
    sound: Box<dyn Sound>,
    volume: Arc<AtomicU8>,
    ringing: bool,
}

impl SoundRinger {
    pub fn new(sound: Box<dyn Sound>) -> Self {
        Self { sound, volume: Arc::default(), ringing: false }
    }
}

impl Ringer for SoundRinger {
    fn ring(&mut self, volume: Volume) {
        self.volume.store(volume.as_percent(), Ordering::Relaxed);
        if !self.ringing {
            self.sound.play(Box::new(Ringtone::new(self.sound.sample_rate_hz(), Arc::clone(&self.volume))));
            self.ringing = true;
        }
    }

    fn silence(&mut self) {
        if self.ringing {
            self.sound.stop();
            self.ringing = false;
        }
    }
}

struct Ringtone {
    sample_rate_hz: u32,
    volume: Arc<AtomicU8>,
    note: usize,
    sample: u32,
}

impl Ringtone {
    fn new(sample_rate_hz: u32, volume: Arc<AtomicU8>) -> Self {
        Self { sample_rate_hz, volume, note: 0, sample: 0 }
    }

    fn samples_in(&self, ms: u32) -> u32 {
        self.sample_rate_hz * ms / 1000
    }
}

impl Iterator for Ringtone {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let (tone, ms) = PHRASE.get(self.note).copied().unwrap_or((None, 0));
        let length = self.samples_in(ms);
        if self.sample >= length {
            self.note = (self.note + 1) % PHRASE.len();
            self.sample = 0;
            return Some(0);
        }
        let n = self.sample;
        self.sample += 1;
        let Some(hz) = tone else {
            return Some(0);
        };
        let fade = self.samples_in(FADE_MS).max(1);
        let envelope = n.min(length - 1 - n).min(fade) as f32 / fade as f32;
        let volume = f32::from(self.volume.load(Ordering::Relaxed)) / 100.0;
        let wave = (TAU * hz * n as f32 / self.sample_rate_hz as f32).sin();
        Some((wave * envelope * volume * f32::from(i16::MAX)) as i16)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use domain::sound::Samples;

    use super::*;

    const RATE: u32 = 8_000;

    fn ringtone(volume: u8) -> Ringtone {
        Ringtone::new(RATE, Arc::new(AtomicU8::new(volume)))
    }

    fn phrase_samples() -> usize {
        PHRASE.iter().map(|&(_, ms)| (RATE * ms / 1000) as usize + 1).sum()
    }

    #[test]
    fn the_phrase_comes_round_again_and_reaches_full_scale() {
        let samples: Vec<i16> = ringtone(100).take(2 * phrase_samples()).collect();
        let (first, second) = samples.split_at(phrase_samples());
        assert_eq!(first, second);
        assert!(samples.iter().any(|s| s.abs() > i16::MAX / 2));
    }

    #[test]
    fn half_the_volume_is_half_as_loud() {
        let peak = |volume| ringtone(volume).take(phrase_samples()).map(|s| s.abs()).max().unwrap();
        assert!((peak(50) - peak(100) / 2).abs() <= 1);
    }

    #[derive(Clone, Default)]
    struct FakeSound {
        asked: Arc<Mutex<Vec<&'static str>>>,
        playing: Arc<Mutex<Option<Samples>>>,
    }

    impl Sound for FakeSound {
        fn sample_rate_hz(&self) -> u32 {
            RATE
        }
        fn play(&mut self, samples: Samples) {
            self.asked.lock().unwrap().push("play");
            *self.playing.lock().unwrap() = Some(samples);
        }
        fn stop(&mut self) {
            self.asked.lock().unwrap().push("stop");
        }
    }

    #[test]
    fn ringing_louder_plays_once_and_the_ringtone_follows_the_volume() {
        let sound = FakeSound::default();
        let mut ringer = SoundRinger::new(Box::new(sound.clone()));
        ringer.ring(Volume::percent(10));
        ringer.ring(Volume::percent(100));
        let peak = sound.playing.lock().unwrap().take().unwrap().take(phrase_samples()).map(|s| s.abs()).max().unwrap();
        assert!(peak > i16::MAX / 2);
        ringer.silence();
        ringer.silence();
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "stop"]);
    }
}
