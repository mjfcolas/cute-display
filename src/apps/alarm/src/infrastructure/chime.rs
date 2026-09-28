use core::f32::consts::TAU;

/// C5 E5 G5 C6, then silence; in milliseconds.
const PHRASE: [(Option<f32>, u32); 5] = [(Some(523.25), 150), (Some(659.25), 150), (Some(783.99), 150), (Some(1046.5), 300), (None, 700)];
const FADE_MS: u32 = 10;

/// The ringtone played without a file, at full scale, round and round.
pub struct Chime {
    sample_rate_hz: u32,
    note: usize,
    sample: u32,
}

impl Chime {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self { sample_rate_hz, note: 0, sample: 0 }
    }

    fn samples_in(&self, ms: u32) -> u32 {
        self.sample_rate_hz * ms / 1000
    }
}

impl Iterator for Chime {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let (tone, ms) = PHRASE.get(self.note).copied().unwrap_or((None, 0));
        let length = self.samples_in(ms);
        if self.sample >= length {
            self.note = (self.note + 1) % PHRASE.len();
            self.sample = 0;
            return Some(0);
        }
        let n = self.sample;
        self.sample += 1;
        let Some(hz) = tone else {
            return Some(0);
        };
        let fade = self.samples_in(FADE_MS).max(1);
        let envelope = n.min(length - 1 - n).min(fade) as f32 / fade as f32;
        let wave = (TAU * hz * n as f32 / self.sample_rate_hz as f32).sin();
        Some((wave * envelope * f32::from(i16::MAX)) as i16)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const RATE: u32 = 8_000;

    pub fn phrase_samples() -> usize {
        PHRASE.iter().map(|&(_, ms)| (RATE * ms / 1000) as usize + 1).sum()
    }

    #[test]
    fn the_phrase_comes_round_again_and_reaches_full_scale() {
        let samples: Vec<i16> = Chime::new(RATE).take(2 * phrase_samples()).collect();
        let (first, second) = samples.split_at(phrase_samples());
        assert_eq!(first, second);
        assert!(samples.iter().any(|s| s.abs() > i16::MAX / 2));
    }
}
