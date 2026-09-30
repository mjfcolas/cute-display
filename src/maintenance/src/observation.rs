//! What the app image does with its lights and its speaker, for a computer that tests it.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use hal::audio::Speaker;
use hal::light::{Brightness, DimmableLight};
use hal::Fault;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightName {
    FrontLight,
    ReadingLamp,
}

/// Clones share their state with the lights and the speaker they observe.
#[derive(Clone, Default)]
pub struct Observation {
    front_light: Arc<AtomicU8>,
    reading_lamp: Arc<AtomicU8>,
    playing: Arc<AtomicBool>,
}

impl Observation {
    pub fn light<L: DimmableLight>(&self, name: LightName, light: L) -> ObservedLight<L> {
        let percent = self.percent(name).clone();
        percent.store(light.brightness().as_percent(), Ordering::Relaxed);
        ObservedLight { light, percent }
    }

    pub fn speaker<S: Speaker>(&self, speaker: S) -> ObservedSpeaker<S> {
        ObservedSpeaker { speaker, playing: self.playing.clone() }
    }

    pub fn brightness(&self, name: LightName) -> Brightness {
        Brightness::percent(self.percent(name).load(Ordering::Relaxed))
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    fn percent(&self, name: LightName) -> &Arc<AtomicU8> {
        match name {
            LightName::FrontLight => &self.front_light,
            LightName::ReadingLamp => &self.reading_lamp,
        }
    }
}

pub struct ObservedLight<L> {
    light: L,
    percent: Arc<AtomicU8>,
}

impl<L: DimmableLight> DimmableLight for ObservedLight<L> {
    fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault> {
        self.light.set_brightness(brightness)?;
        self.percent.store(brightness.as_percent(), Ordering::Relaxed);
        Ok(())
    }

    fn brightness(&self) -> Brightness {
        self.light.brightness()
    }
}

pub struct ObservedSpeaker<S> {
    speaker: S,
    playing: Arc<AtomicBool>,
}

impl<S: Speaker> Speaker for ObservedSpeaker<S> {
    fn sample_rate_hz(&self) -> u32 {
        self.speaker.sample_rate_hz()
    }

    fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
        self.playing.store(true, Ordering::Relaxed);
        let played = self.speaker.play(samples);
        self.playing.store(false, Ordering::Relaxed);
        played
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct Dimmer(Brightness);
    impl DimmableLight for Dimmer {
        fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault> {
            self.0 = brightness;
            Ok(())
        }
        fn brightness(&self) -> Brightness {
            self.0
        }
    }

    struct Broken;
    impl DimmableLight for Broken {
        fn set_brightness(&mut self, _: Brightness) -> Result<(), Fault> {
            Err(Fault::new("LEDC"))
        }
        fn brightness(&self) -> Brightness {
            Brightness::OFF
        }
    }

    #[test]
    fn each_light_is_seen_at_the_brightness_it_was_set_to() {
        let observation = Observation::default();
        let mut front = observation.light(LightName::FrontLight, Dimmer(Brightness::percent(10)));
        let _lamp = observation.light(LightName::ReadingLamp, Dimmer(Brightness::OFF));
        assert_eq!(observation.brightness(LightName::FrontLight), Brightness::percent(10));
        front.set_brightness(Brightness::percent(40)).unwrap();
        assert_eq!(observation.brightness(LightName::FrontLight), Brightness::percent(40));
        assert_eq!(observation.brightness(LightName::ReadingLamp), Brightness::OFF);
    }

    #[test]
    fn a_light_that_failed_is_seen_as_it_was() {
        let observation = Observation::default();
        let mut lamp = observation.light(LightName::ReadingLamp, Broken);
        assert!(lamp.set_brightness(Brightness::FULL).is_err());
        assert_eq!(observation.brightness(LightName::ReadingLamp), Brightness::OFF);
    }

    struct Listening(Observation, Arc<Mutex<Vec<bool>>>);
    impl Speaker for Listening {
        fn sample_rate_hz(&self) -> u32 {
            8000
        }
        fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
            samples.for_each(drop);
            self.1.lock().unwrap().push(self.0.is_playing());
            Ok(())
        }
    }

    #[test]
    fn the_speaker_is_playing_only_while_it_plays() {
        let observation = Observation::default();
        let heard = Arc::new(Mutex::new(Vec::new()));
        let mut speaker = observation.speaker(Listening(observation.clone(), heard.clone()));
        speaker.play(&mut [1, 2, 3].into_iter()).unwrap();
        assert_eq!(*heard.lock().unwrap(), [true]);
        assert!(!observation.is_playing());
    }
}
