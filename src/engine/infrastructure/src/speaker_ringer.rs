//! The alarm's ring on the speaker: a rising arpeggio and a pause, over and over, as loud
//! as the alarm says. Playing blocks, so a player of its own does it on a thread of its
//! own; the ringer only tells it what to do.

use core::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use domain::alarm::{Ringer, Volume};
use hal::audio::Speaker;

/// C5 E5 G5 C6, then silence; in milliseconds.
const PHRASE: [(Option<f32>, u32); 5] = [(Some(523.25), 150), (Some(659.25), 150), (Some(783.99), 150), (Some(1046.5), 300), (None, 700)];
/// Of full scale, at full volume. The amplifier is loud: the hardware test's chime is
/// comfortable at 0.08, a pure tone at 0.25 already hurts.
const LOUDEST: f32 = 0.2;
const FADE_MS: u32 = 10;

#[derive(Default)]
struct Command {
    ringing: AtomicBool,
    volume_percent: AtomicU8,
}

/// What the alarm holds: every call returns at once.
pub struct SpeakerRinger {
    command: Arc<Command>,
    wake: Sender<()>,
}

/// Plays on the calling thread whatever the ringer asks for.
pub struct RingtonePlayer<S> {
    speaker: S,
    command: Arc<Command>,
    woken: Receiver<()>,
}

pub fn ringer<S: Speaker>(speaker: S) -> (SpeakerRinger, RingtonePlayer<S>) {
    let (wake, woken) = mpsc::channel();
    let command = Arc::new(Command::default());
    (SpeakerRinger { command: Arc::clone(&command), wake }, RingtonePlayer { speaker, command, woken })
}

impl Ringer for SpeakerRinger {
    fn ring(&mut self, volume: Volume) {
        self.command.volume_percent.store(volume.as_percent(), Ordering::Relaxed);
        if !self.command.ringing.swap(true, Ordering::Relaxed) && self.wake.send(()).is_err() {
            log::warn!("alarm: the speaker's player is gone");
        }
    }

    fn silence(&mut self) {
        self.command.ringing.store(false, Ordering::Relaxed);
    }
}

impl<S: Speaker> RingtonePlayer<S> {
    /// Returns when the ringer is gone.
    pub fn run(mut self) {
        for () in &self.woken {
            while self.command.ringing.load(Ordering::Relaxed) {
                let mut ringtone = Ringtone::new(self.speaker.sample_rate_hz(), &self.command);
                if let Err(fault) = self.speaker.play(&mut ringtone) {
                    log::warn!("alarm: {fault}");
                    self.command.ringing.store(false, Ordering::Relaxed);
                }
            }
        }
    }
}

/// The phrase over and over, until the ringer is silenced.
struct Ringtone<'a> {
    sample_rate_hz: u32,
    command: &'a Command,
    note: usize,
    sample: u32,
}

impl<'a> Ringtone<'a> {
    fn new(sample_rate_hz: u32, command: &'a Command) -> Self {
        Self { sample_rate_hz, command, note: 0, sample: 0 }
    }

    fn samples_in(&self, ms: u32) -> u32 {
        self.sample_rate_hz * ms / 1000
    }
}

impl Iterator for Ringtone<'_> {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if !self.command.ringing.load(Ordering::Relaxed) {
            return None;
        }
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
        let volume = f32::from(self.command.volume_percent.load(Ordering::Relaxed)) / 100.0;
        let wave = (TAU * hz * n as f32 / self.sample_rate_hz as f32).sin();
        Some((wave * envelope * volume * LOUDEST * f32::from(i16::MAX)) as i16)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::thread;

    use hal::Fault;

    use super::*;

    const RATE: u32 = 8_000;

    fn command(volume: u8) -> Command {
        let command = Command::default();
        command.ringing.store(true, Ordering::Relaxed);
        command.volume_percent.store(volume, Ordering::Relaxed);
        command
    }

    fn phrase_samples() -> usize {
        PHRASE.iter().map(|&(_, ms)| (RATE * ms / 1000) as usize + 1).sum()
    }

    #[test]
    fn the_phrase_comes_round_again_and_stays_under_the_loudest() {
        let command = command(100);
        let samples: Vec<i16> = Ringtone::new(RATE, &command).take(2 * phrase_samples()).collect();
        let (first, second) = samples.split_at(phrase_samples());
        assert_eq!(first, second);
        let ceiling = (LOUDEST * f32::from(i16::MAX)) as i16 + 1;
        assert!(samples.iter().all(|s| s.abs() <= ceiling));
        assert!(samples.iter().any(|s| s.abs() > ceiling / 2));
    }

    #[test]
    fn half_the_volume_is_half_as_loud() {
        let (full, half) = (command(100), command(50));
        let peak = |command: &Command| Ringtone::new(RATE, command).take(phrase_samples()).map(|s| s.abs()).max().unwrap();
        assert!((peak(&half) - peak(&full) / 2).abs() <= 1);
    }

    #[test]
    fn silenced_it_ends() {
        let command = command(100);
        let mut ringtone = Ringtone::new(RATE, &command);
        assert!(ringtone.next().is_some());
        command.ringing.store(false, Ordering::Relaxed);
        assert_eq!(ringtone.next(), None);
    }

    /// Counts the samples played, and silences the ringer after `stop_after` of them.
    struct FakeSpeaker {
        played: Arc<Mutex<usize>>,
        stop_after: usize,
    }

    impl Speaker for FakeSpeaker {
        fn sample_rate_hz(&self) -> u32 {
            RATE
        }
        fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
            let mut played = self.played.lock().unwrap();
            for _ in samples.take(self.stop_after) {
                *played += 1;
            }
            Ok(())
        }
    }

    #[test]
    fn the_player_rings_when_asked_until_silenced() {
        let played = Arc::new(Mutex::new(0));
        let (mut ringer, player) = ringer(FakeSpeaker { played: Arc::clone(&played), stop_after: 100 });
        let command = Arc::clone(&ringer.command);
        let playing = thread::spawn(move || player.run());
        ringer.ring(Volume::percent(10));
        while *played.lock().unwrap() < 1000 {
            thread::yield_now();
        }
        ringer.silence();
        drop(ringer);
        playing.join().unwrap();
        assert!(!command.ringing.load(Ordering::Relaxed));
    }
}
