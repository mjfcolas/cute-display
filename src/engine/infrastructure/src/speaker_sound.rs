use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use domain::sound::{Samples, Sound};
use hal::audio::Speaker;

/// Of full scale, for the loudest a sound may be. The amplifier is loud: the hardware
/// test's chime is comfortable at 0.08, a pure tone at 0.25 already hurts.
const LOUDEST: f32 = 0.2;

#[derive(Clone)]
pub struct SpeakerSound {
    sample_rate_hz: u32,
    /// Moves on at every play and every stop: a sound plays while it is its own. 32 bits,
    /// the most the ESP32-S3 has atomics for; wrapping round is harmless.
    turn: Arc<AtomicU32>,
    next: Sender<(u32, Samples)>,
}

pub struct SoundPlayer<S> {
    speaker: S,
    turn: Arc<AtomicU32>,
    asked: Receiver<(u32, Samples)>,
}

pub fn sound<S: Speaker>(speaker: S) -> (SpeakerSound, SoundPlayer<S>) {
    let (next, asked) = mpsc::channel();
    let turn = Arc::new(AtomicU32::new(0));
    (SpeakerSound { sample_rate_hz: speaker.sample_rate_hz(), turn: Arc::clone(&turn), next }, SoundPlayer { speaker, turn, asked })
}

impl Sound for SpeakerSound {
    fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    fn play(&mut self, samples: Samples) {
        let turn = self.turn.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        if self.next.send((turn, samples)).is_err() {
            log::warn!("sound: the speaker's player is gone");
        }
    }

    fn stop(&mut self) {
        self.turn.fetch_add(1, Ordering::Relaxed);
    }
}

impl<S: Speaker> SoundPlayer<S> {
    pub fn run(mut self) {
        for (turn, samples) in &self.asked {
            let mut playing = Playing { samples, turn, now: &self.turn };
            if let Err(fault) = self.speaker.play(&mut playing) {
                log::warn!("sound: {fault}");
            }
        }
    }
}

struct Playing<'a> {
    samples: Samples,
    turn: u32,
    now: &'a AtomicU32,
}

impl Iterator for Playing<'_> {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if self.now.load(Ordering::Relaxed) != self.turn {
            return None;
        }
        self.samples.next().map(|sample| (f32::from(sample) * LOUDEST) as i16)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use hal_testing::audio::StubSpeaker;

    use super::*;

    fn played_after(ask: impl FnOnce(&mut SpeakerSound)) -> Vec<i16> {
        let speaker = StubSpeaker::default();
        let (mut sound, player) = sound(speaker.clone());
        assert_eq!(sound.sample_rate_hz(), StubSpeaker::SAMPLE_RATE_HZ);
        ask(&mut sound);
        drop(sound);
        player.run();
        speaker.samples()
    }

    #[test]
    fn a_sound_plays_to_its_end_scaled_to_the_loudest() {
        let played = played_after(|sound| sound.play(Box::new([i16::MAX, 0, i16::MIN].into_iter())));
        let scaled = |sample: i16| (f32::from(sample) * LOUDEST) as i16;
        assert_eq!(played, [scaled(i16::MAX), 0, scaled(i16::MIN)]);
    }

    #[test]
    fn a_new_sound_cuts_short_the_one_playing() {
        let played = played_after(|sound| {
            sound.play(Box::new(std::iter::repeat(1000)));
            sound.play(Box::new([2000].into_iter()));
        });
        assert_eq!(played, [(2000.0 * LOUDEST) as i16]);
    }

    #[test]
    fn a_sound_stopped_while_it_plays_ends() {
        let speaker = StubSpeaker::default();
        let (mut sound, player) = sound(speaker.clone());
        let playing = thread::spawn(move || player.run());
        sound.play(Box::new(std::iter::repeat(1000)));
        while speaker.samples_played() < 1000 {
            thread::yield_now();
        }
        sound.stop();
        drop(sound);
        playing.join().unwrap();
    }
}
