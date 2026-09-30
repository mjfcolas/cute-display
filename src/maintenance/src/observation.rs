//! What the app image does with its glass, its lights and its speaker, for a computer that
//! tests it.

use core::ops::DerefMut;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use hal::audio::Speaker;
use hal::display::{EpaperDisplay, Frame, Redraw, Refreshed, FRAME_BYTES};
use hal::light::{Brightness, DimmableLight};
use hal::Fault;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightName {
    FrontLight,
    ReadingLamp,
}

type LastFrame = Box<dyn DerefMut<Target = [u8; FRAME_BYTES]> + Send>;

#[derive(Default)]
struct Glass {
    times_shown: u64,
    last_frame: Option<LastFrame>,
}

/// Clones share their state with the glass, the lights and the speaker they observe.
#[derive(Clone, Default)]
pub struct Observation {
    glass: Arc<Mutex<Glass>>,
    front_light: Arc<AtomicU8>,
    reading_lamp: Arc<AtomicU8>,
    playing: Arc<AtomicBool>,
}

impl Observation {
    /// `last_frame` is given, so that the board can keep it in PSRAM.
    pub fn panel<P: EpaperDisplay>(
        &self,
        panel: P,
        last_frame: impl DerefMut<Target = [u8; FRAME_BYTES]> + Send + 'static,
    ) -> ObservedPanel<P> {
        if let Ok(mut glass) = self.glass.lock() {
            glass.last_frame = Some(Box::new(last_frame));
        }
        ObservedPanel { panel, glass: self.glass.clone() }
    }

    pub fn light<L: DimmableLight>(&self, name: LightName, light: L) -> ObservedLight<L> {
        let percent = self.percent(name).clone();
        percent.store(light.brightness().as_percent(), Ordering::Relaxed);
        ObservedLight { light, percent }
    }

    pub fn speaker<S: Speaker>(&self, speaker: S) -> ObservedSpeaker<S> {
        ObservedSpeaker { speaker, playing: self.playing.clone() }
    }

    /// Copies the bytes of the last frame shown from `offset` into `part`, laid out as
    /// `Frame::as_bytes` does, and gives how many times the app has shown something. Nothing
    /// before it did, or for a part outside the frame.
    pub fn copy_screen(&self, offset: usize, part: &mut [u8]) -> Option<u64> {
        let glass = self.glass.lock().ok()?;
        let last_frame = glass.last_frame.as_ref().filter(|_| glass.times_shown > 0)?;
        part.copy_from_slice(last_frame.get(offset..offset.checked_add(part.len())?)?);
        Some(glass.times_shown)
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

pub struct ObservedPanel<P> {
    panel: P,
    glass: Arc<Mutex<Glass>>,
}

impl<P: EpaperDisplay> EpaperDisplay for ObservedPanel<P> {
    fn show(&mut self, frame: &Frame, redraw: Redraw) -> Result<Refreshed, Fault> {
        let refreshed = self.panel.show(frame, redraw)?;
        if let Ok(mut glass) = self.glass.lock() {
            if let Some(last_frame) = glass.last_frame.as_mut() {
                // Not `**last_frame = *frame.as_bytes()`: that goes through the stack in
                // debug builds, and overflowed the simulator's UI thread.
                last_frame.copy_from_slice(frame.as_bytes());
            }
            glass.times_shown += 1;
        }
        Ok(refreshed)
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

    use hal_testing::display::StubPanel;
    use hal_testing::light::FakeLight;

    use super::*;

    struct StubFailingLight;
    impl DimmableLight for StubFailingLight {
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
        let mut front = observation.light(LightName::FrontLight, FakeLight::at(Brightness::percent(10)));
        let _lamp = observation.light(LightName::ReadingLamp, FakeLight::default());
        assert_eq!(observation.brightness(LightName::FrontLight), Brightness::percent(10));
        front.set_brightness(Brightness::percent(40)).unwrap();
        assert_eq!(observation.brightness(LightName::FrontLight), Brightness::percent(40));
        assert_eq!(observation.brightness(LightName::ReadingLamp), Brightness::OFF);
    }

    #[test]
    fn a_light_that_failed_is_seen_as_it_was() {
        let observation = Observation::default();
        let mut lamp = observation.light(LightName::ReadingLamp, StubFailingLight);
        assert!(lamp.set_brightness(Brightness::FULL).is_err());
        assert_eq!(observation.brightness(LightName::ReadingLamp), Brightness::OFF);
    }

    struct StubFailingPanel;
    impl EpaperDisplay for StubFailingPanel {
        fn show(&mut self, _: &Frame, _: Redraw) -> Result<Refreshed, Fault> {
            Err(Fault::new("the panel stayed busy"))
        }
    }

    fn last_frame() -> Box<[u8; FRAME_BYTES]> {
        vec![0; FRAME_BYTES].into_boxed_slice().try_into().unwrap()
    }

    #[test]
    fn the_last_frame_shown_is_copied_a_part_at_a_time_and_counted() {
        let observation = Observation::default();
        let mut panel = observation.panel(StubPanel::default(), last_frame());
        let mut part = [0; 8];
        assert_eq!(observation.copy_screen(0, &mut part), None);
        let mut frame = Frame::blank();
        frame.set_ink(8 * 8 + 3, 0, true);
        panel.show(&Frame::blank(), Redraw::Whole).unwrap();
        panel.show(&frame, Redraw::Changes).unwrap();
        assert_eq!(observation.copy_screen(8, &mut part), Some(2));
        assert_eq!(part, frame.as_bytes()[8..16]);
        assert_eq!(observation.copy_screen(FRAME_BYTES - 4, &mut part), None, "past the end");
    }

    #[test]
    fn a_frame_the_panel_did_not_show_is_not_seen() {
        let observation = Observation::default();
        let mut panel = observation.panel(StubFailingPanel, last_frame());
        assert!(panel.show(&Frame::blank(), Redraw::Whole).is_err());
        assert_eq!(observation.copy_screen(0, &mut [0; 8]), None);
    }

    struct StubPlayingSpeaker(Observation, Arc<Mutex<Vec<bool>>>);
    impl Speaker for StubPlayingSpeaker {
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
        let mut speaker = observation.speaker(StubPlayingSpeaker(observation.clone(), heard.clone()));
        speaker.play(&mut [1, 2, 3].into_iter()).unwrap();
        assert_eq!(*heard.lock().unwrap(), [true]);
        assert!(!observation.is_playing());
    }
}
