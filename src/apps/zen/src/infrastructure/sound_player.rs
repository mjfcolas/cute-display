use domain::files::Files;
use domain::sound::{Samples, Sound};
use sound_files::{sound_files_in, Recording, SharedFiles, VolumeKnob};

use crate::domain::zen_sessions::{Player, Soundscape, Volume};
use crate::infrastructure::brown_noise::BrownNoise;

pub const SOUNDS_DIRECTORY: &str = "sounds";

pub struct SoundPlayer {
    sound: Box<dyn Sound>,
    files: SharedFiles,
    volume: VolumeKnob,
    playing: Option<Soundscape>,
}

impl SoundPlayer {
    pub fn new(sound: Box<dyn Sound>, files: Box<dyn Files>) -> Self {
        Self { sound, files: SharedFiles::new(files), volume: VolumeKnob::default(), playing: None }
    }

    /// A file that cannot be played gives way to brown noise: the session goes on anyway.
    fn samples_of(&self, soundscape: &Soundscape) -> Samples {
        let samples: Samples = match soundscape {
            Soundscape::BrownNoise => Box::new(BrownNoise::new()),
            Soundscape::Recorded(name) => {
                let recording = Recording::new(Box::new(self.files.clone()), format!("{SOUNDS_DIRECTORY}/{name}"), self.sound.sample_rate_hz());
                Box::new(recording.chain(BrownNoise::new()))
            }
        };
        self.volume.applied_to(samples)
    }
}

impl Player for SoundPlayer {
    fn recordings(&self) -> Vec<String> {
        sound_files_in(&self.files, SOUNDS_DIRECTORY).unwrap_or_else(|unavailable| {
            log::warn!("zen: {unavailable}; brown noise alone");
            Vec::new()
        })
    }

    fn play(&mut self, soundscape: &Soundscape, volume: Volume) {
        self.volume.turn_to(volume);
        if self.playing.as_ref() != Some(soundscape) {
            self.sound.play(self.samples_of(soundscape));
            self.playing = Some(soundscape.clone());
        }
    }

    fn silence(&mut self) {
        if self.playing.take().is_some() {
            self.sound.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain_testing::files::FakeFiles;

    use super::*;

    const RATE: u32 = 44_100;

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
        fn played(&self, count: usize) -> Vec<i16> {
            self.playing.lock().unwrap().as_mut().unwrap().take(count).collect()
        }
    }

    fn rain() -> Soundscape {
        Soundscape::Recorded("Rain.wav".into())
    }

    #[test]
    fn playing_on_plays_once_and_the_sound_follows_the_volume() {
        let sound = StubSound::default();
        let mut player = SoundPlayer::new(Box::new(sound.clone()), Box::new(FakeFiles::default()));
        player.play(&Soundscape::BrownNoise, Volume::percent(100));
        let loud = sound.played(1000);
        player.play(&Soundscape::BrownNoise, Volume::percent(0));
        assert!(loud.iter().any(|&sample| sample != 0));
        assert!(sound.played(1000).iter().all(|&sample| sample == 0));
        player.silence();
        player.silence();
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "stop"]);
    }

    #[test]
    fn another_sound_plays_in_place_of_the_one_playing() {
        let sound = StubSound::default();
        let mut player = SoundPlayer::new(Box::new(sound.clone()), Box::new(FakeFiles::default()));
        player.play(&Soundscape::BrownNoise, Volume::percent(30));
        player.play(&rain(), Volume::percent(30));
        player.play(&rain(), Volume::percent(40));
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "play"]);
    }

    #[test]
    fn a_file_that_cannot_be_played_plays_brown_noise() {
        let sound = StubSound::default();
        let mut player = SoundPlayer::new(Box::new(sound.clone()), Box::new(FakeFiles::default()));
        player.play(&rain(), Volume::percent(100));
        assert_eq!(sound.played(1000), BrownNoise::new().take(1000).collect::<Vec<_>>());
    }

    #[test]
    fn the_recordings_are_the_sound_files_in_their_directory() {
        let files = FakeFiles::default();
        for name in ["sounds/Rain.wav", "sounds/Zen.mp3", "sounds/notes.txt", "zen.conf"] {
            files.put(name, b"");
        }
        let player = SoundPlayer::new(Box::new(StubSound::default()), Box::new(files));
        assert_eq!(player.recordings(), ["Rain.wav", "Zen.mp3"]);
    }
}
