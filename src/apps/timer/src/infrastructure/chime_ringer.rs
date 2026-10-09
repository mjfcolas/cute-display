use domain::sound::{Samples, Sound};
use sound_files::{Chime, VolumeKnob};

use crate::domain::timer::{Ringer, Volume};

/// Phrases of the chime between two periods of a pomodoro.
const CHIME_PHRASES: usize = 2;

pub struct ChimeRinger {
    sound: Box<dyn Sound>,
    volume: VolumeKnob,
    ringing: bool,
}

impl ChimeRinger {
    pub fn new(sound: Box<dyn Sound>) -> Self {
        Self { sound, volume: VolumeKnob::default(), ringing: false }
    }

    fn play(&mut self, samples: Samples, volume: Volume) {
        self.volume.turn_to(volume);
        self.sound.play(self.volume.applied_to(samples));
    }
}

impl Ringer for ChimeRinger {
    fn ring(&mut self, volume: Volume) {
        if self.ringing {
            self.volume.turn_to(volume);
        } else {
            self.play(Box::new(Chime::new(self.sound.sample_rate_hz())), volume);
            self.ringing = true;
        }
    }

    fn chime(&mut self, volume: Volume) {
        let chime = Chime::new(self.sound.sample_rate_hz());
        let phrases = CHIME_PHRASES * chime.phrase_samples();
        self.play(Box::new(chime.take(phrases)), volume);
        self.ringing = false;
    }

    /// A chime is left to end by itself: stopping the speaker after it would stop whatever
    /// plays next.
    fn silence(&mut self) {
        if std::mem::take(&mut self.ringing) {
            self.sound.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    const RATE: u32 = 8_000;

    #[derive(Clone, Default)]
    struct StubSound {
        asked: Arc<Mutex<Vec<&'static str>>>,
        playing: Arc<Mutex<Option<Samples>>>,
    }

    impl Sound for StubSound {
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

    impl StubSound {
        fn peak_of(&self, count: usize) -> i16 {
            self.playing.lock().unwrap().as_mut().unwrap().take(count).map(i16::saturating_abs).max().unwrap()
        }

        fn left(&self) -> usize {
            self.playing.lock().unwrap().take().unwrap().count()
        }
    }

    fn phrase() -> usize {
        Chime::new(RATE).phrase_samples()
    }

    #[test]
    fn ringing_louder_plays_once_and_the_sound_follows_the_volume() {
        let sound = StubSound::default();
        let mut ringer = ChimeRinger::new(Box::new(sound.clone()));
        ringer.ring(Volume::percent(50));
        let half = sound.peak_of(phrase());
        ringer.ring(Volume::percent(100));
        assert!((sound.peak_of(phrase()) / 2 - half).abs() <= 1);
        ringer.silence();
        ringer.silence();
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "stop"]);
    }

    #[test]
    fn a_chime_is_two_phrases_and_left_to_end() {
        let sound = StubSound::default();
        let mut ringer = ChimeRinger::new(Box::new(sound.clone()));
        ringer.chime(Volume::percent(50));
        ringer.silence();
        assert_eq!(*sound.asked.lock().unwrap(), ["play"]);
        assert_eq!(sound.left(), 2 * phrase());
    }

    #[test]
    fn a_chime_takes_over_a_ringing_which_rings_again_from_the_start() {
        let sound = StubSound::default();
        let mut ringer = ChimeRinger::new(Box::new(sound.clone()));
        ringer.ring(Volume::percent(10));
        ringer.chime(Volume::percent(50));
        ringer.ring(Volume::percent(10));
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "play", "play"]);
    }
}
