use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use domain::fetch::Unavailable;
use domain::files::Files;
use domain::sound::{Samples, Sound};

use crate::domain::alarm_clock::{Ringer, Ringtone, Volume};
use crate::infrastructure::chime::Chime;
use crate::infrastructure::recording::Recording;

const RINGTONES_DIRECTORY: &str = "ringtones";
const RECORDING_EXTENSION: &str = ".mp3";

pub struct SoundRinger {
    sound: Box<dyn Sound>,
    files: SharedFiles,
    volume: Arc<AtomicU8>,
    ringing: Option<Ringtone>,
}

impl SoundRinger {
    pub fn new(sound: Box<dyn Sound>, files: Box<dyn Files>) -> Self {
        Self { sound, files: SharedFiles(Arc::new(Mutex::new(files))), volume: Arc::default(), ringing: None }
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
        Box::new(AtVolume { samples, volume: Arc::clone(&self.volume) })
    }
}

impl Ringer for SoundRinger {
    fn recordings(&self) -> Vec<String> {
        let names = self.files.names_in(RINGTONES_DIRECTORY).unwrap_or_else(|unavailable| {
            log::warn!("alarm: {unavailable}; the chime alone");
            Vec::new()
        });
        names.into_iter().filter(|name| name.to_lowercase().ends_with(RECORDING_EXTENSION)).collect()
    }

    fn ring(&mut self, ringtone: &Ringtone, volume: Volume) {
        self.volume.store(volume.as_percent(), Ordering::Relaxed);
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

/// The alarm's files, for the ringer and for the recording playing on the speaker's thread.
#[derive(Clone)]
struct SharedFiles(Arc<Mutex<Box<dyn Files>>>);

impl SharedFiles {
    fn with<T>(&self, use_files: impl FnOnce(&dyn Files) -> T) -> T {
        use_files(self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_ref())
    }
}

impl Files for SharedFiles {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        self.with(|files| files.read(name))
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable> {
        self.with(|files| files.write(name, text))
    }

    fn read_bytes(&self, name: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Unavailable> {
        self.with(|files| files.read_bytes(name, offset, max_bytes))
    }

    fn names_in(&self, directory: &str) -> Result<Vec<String>, Unavailable> {
        self.with(|files| files.names_in(directory))
    }
}

struct AtVolume {
    samples: Samples,
    volume: Arc<AtomicU8>,
}

impl Iterator for AtVolume {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let volume = i32::from(self.volume.load(Ordering::Relaxed));
        self.samples.next().map(|sample| i16::try_from(i32::from(sample) * volume / 100).unwrap_or(sample))
    }
}

#[cfg(test)]
mod tests {
    use domain_testing::files::FakeFiles;

    use super::*;
    use crate::infrastructure::chime::tests::{phrase_samples, RATE};

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
    fn the_recordings_are_the_mp3_files_in_their_directory() {
        let files = FakeFiles::default();
        for name in ["ringtones/Zen.mp3", "ringtones/Default.MP3", "ringtones/notes.txt", "alarm.conf"] {
            files.put(name, b"");
        }
        let ringer = SoundRinger::new(Box::new(StubSound::default()), Box::new(files));
        assert_eq!(ringer.recordings(), ["Default.MP3", "Zen.mp3"]);
    }
}
