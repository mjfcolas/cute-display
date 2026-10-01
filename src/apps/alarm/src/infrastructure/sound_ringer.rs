use domain::files::Files;
use domain::sound::{Samples, Sound};
use sound_files::{sound_files_in, Chime, Recording, SharedFiles, VolumeKnob};

use crate::domain::alarm_clock::{Ringer, Ringtone, Volume};

const RINGTONES_DIRECTORY: &str = "ringtones";

pub struct SoundRinger {
    sound: Box<dyn Sound>,
    files: SharedFiles,
    volume: VolumeKnob,
    ringing: Option<Ringtone>,
}

impl SoundRinger {
    pub fn new(sound: Box<dyn Sound>, files: Box<dyn Files>) -> Self {
        Self { sound, files: SharedFiles::new(files), volume: VolumeKnob::default(), ringing: None }
    }

    /// A recording that cannot be played gives way to the chime: the alarm rings anyway.
    fn samples_of(&self, ringtone: &Ringtone) -> Samples {
        let rate = self.sound.sample_rate_hz();
        let samples: Samples = match ringtone {
            Ringtone::Chime => Box::new(Chime::new(rate)),
            Ringtone::Recorded(name) => {
                let recording = Recording::new(Box::new(self.files.clone()), format!("{RINGTONES_DIRECTORY}/{name}"), rate);
                Box::new(recording.chain(Chime::new(rate)))
            }
        };
        self.volume.applied_to(samples)
    }
}

impl Ringer for SoundRinger {
    fn recordings(&self) -> Vec<String> {
        sound_files_in(&self.files, RINGTONES_DIRECTORY).unwrap_or_else(|unavailable| {
            log::warn!("alarm: {unavailable}; the chime alone");
            Vec::new()
        })
    }

    fn ring(&mut self, ringtone: &Ringtone, volume: Volume) {
        self.volume.turn_to(volume);
        if self.ringing.as_ref() != Some(ringtone) {
            self.sound.play(self.samples_of(ringtone));
            self.ringing = Some(ringtone.clone());
        }
    }

    fn silence(&mut self) {
        if self.ringing.take().is_some() {
            self.sound.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain_testing::files::FakeFiles;

    use super::*;

    const RATE: u32 = 8_000;

    fn phrase_samples() -> usize {
        Chime::new(RATE).phrase_samples()
    }

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
        fn peak(&self) -> i16 {
            self.playing.lock().unwrap().as_mut().unwrap().take(phrase_samples()).map(|s| s.abs()).max().unwrap()
        }
    }

    fn zen() -> Ringtone {
        Ringtone::Recorded("Zen.mp3".into())
    }

    #[test]
    fn ringing_louder_plays_once_and_the_sound_follows_the_volume() {
        let sound = StubSound::default();
        let mut ringer = SoundRinger::new(Box::new(sound.clone()), Box::new(FakeFiles::default()));
        ringer.ring(&Ringtone::Chime, Volume::percent(50));
        let half = sound.peak();
        ringer.ring(&Ringtone::Chime, Volume::percent(100));
        assert!((sound.peak() / 2 - half).abs() <= 1);
        ringer.silence();
        ringer.silence();
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "stop"]);
    }

    #[test]
    fn another_ringtone_plays_in_place_of_the_one_ringing() {
        let sound = StubSound::default();
        let mut ringer = SoundRinger::new(Box::new(sound.clone()), Box::new(FakeFiles::default()));
        ringer.ring(&Ringtone::Chime, Volume::percent(30));
        ringer.ring(&zen(), Volume::percent(30));
        ringer.ring(&zen(), Volume::percent(30));
        assert_eq!(*sound.asked.lock().unwrap(), ["play", "play"]);
    }

    #[test]
    fn a_ringtone_that_cannot_be_played_rings_the_chime() {
        let sound = StubSound::default();
        let mut ringer = SoundRinger::new(Box::new(sound.clone()), Box::new(FakeFiles::default()));
        ringer.ring(&zen(), Volume::percent(100));
        let chime: Vec<i16> = Chime::new(RATE).take(phrase_samples()).collect();
        let played: Vec<i16> = sound.playing.lock().unwrap().as_mut().unwrap().take(phrase_samples()).collect();
        assert_eq!(played, chime);
    }

    #[test]
    fn the_recordings_are_the_sound_files_in_their_directory() {
        let files = FakeFiles::default();
        for name in ["ringtones/Zen.mp3", "ringtones/Rain.wav", "ringtones/notes.txt", "alarm.conf"] {
            files.put(name, b"");
        }
        let ringer = SoundRinger::new(Box::new(StubSound::default()), Box::new(files));
        assert_eq!(ringer.recordings(), ["Rain.wav", "Zen.mp3"]);
    }
}
