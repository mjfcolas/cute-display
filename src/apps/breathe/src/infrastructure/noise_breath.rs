use domain::sound::Sound;
use sound_files::{BrownNoise, VolumeKnob};

use crate::domain::breathing::{Breath, Volume};

/// Brown noise, turned up and down: a breath like the sea's.
pub struct NoiseBreath {
    sound: Box<dyn Sound>,
    volume: VolumeKnob,
    playing: bool,
}

impl NoiseBreath {
    pub fn new(sound: Box<dyn Sound>) -> Self {
        Self { sound, volume: VolumeKnob::default(), playing: false }
    }
}

impl Breath for NoiseBreath {
    fn sound_at(&mut self, volume: Volume) {
        self.volume.turn_to(volume);
        if !self.playing {
            self.sound.play(self.volume.applied_to(Box::new(BrownNoise::new())));
            self.playing = true;
        }
    }

    fn silence(&mut self) {
        if std::mem::take(&mut self.playing) {
            self.sound.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::sound::Samples;

    use super::*;

    #[derive(Clone, Default)]
    struct StubSound {
        asked: Arc<Mutex<Vec<&'static str>>>,
        playing: Arc<Mutex<Option<Samples>>>,
    }

    impl Sound for StubSound {
        fn sample_rate_hz(&self) -> u32 {
            8_000
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
        fn played(&self, count: usize) -> Vec<i16> {
            self.playing.lock().unwrap().as_mut().unwrap().take(count).collect()
        }
    }

    #[test]
    fn the_noise_plays_once_following_the_volume_until_silenced() {
        let sound = StubSound::default();
        let mut breath = NoiseBreath::new(Box::new(sound.clone()));
        breath.sound_at(Volume::percent(100));
        assert!(sound.played(1000).iter().any(|&sample| sample != 0));
        breath.sound_at(Volume::percent(0));
        assert!(sound.played(1000).iter().all(|&sample| sample == 0));
        breath.silence();
        breath.silence();
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "stop"]);
    }
}
