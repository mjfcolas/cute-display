use std::thread;
use std::time::Duration;

use hal::audio::Speaker;
use hal::Fault;

const SAMPLE_RATE_HZ: u32 = 44_100;
const SLICE: Duration = Duration::from_millis(100);

pub struct LoggedSpeaker;

impl Speaker for LoggedSpeaker {
    fn sample_rate_hz(&self) -> u32 {
        SAMPLE_RATE_HZ
    }

    fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
        log::info!("speaker: playing");
        let per_slice = (SAMPLE_RATE_HZ / 10) as usize;
        let mut loudest = 0;
        loop {
            let slice: Vec<i16> = samples.take(per_slice).collect();
            loudest = slice.iter().map(|s| s.unsigned_abs()).fold(loudest, u16::max);
            if slice.len() < per_slice {
                break;
            }
            thread::sleep(SLICE);
        }
        log::info!("speaker: silent; loudest sample {loudest} of {}", i16::MAX);
        Ok(())
    }
}
