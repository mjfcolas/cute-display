use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use domain::sound::{Samples, Volume};

/// The volume of the samples it is applied to, turned while they play.
#[derive(Clone, Default)]
pub struct VolumeKnob(Arc<AtomicU8>);

impl VolumeKnob {
    pub fn turn_to(&self, volume: Volume) {
        self.0.store(volume.as_percent(), Ordering::Relaxed);
    }

    pub fn applied_to(&self, samples: Samples) -> Samples {
        Box::new(AtVolume { samples, volume: Arc::clone(&self.0) })
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
    use super::*;

    #[test]
    fn the_samples_follow_the_knob_as_they_play() {
        let knob = VolumeKnob::default();
        knob.turn_to(Volume::percent(50));
        let mut samples = knob.applied_to(Box::new(std::iter::repeat(1000)));
        assert_eq!(samples.next(), Some(500));
        knob.turn_to(Volume::percent(10));
        assert_eq!(samples.next(), Some(100));
    }
}
