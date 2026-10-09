/// How much of each new step the walk takes; the rest keeps it from drifting away.
const STEP: f32 = 0.02;
/// Scales the walk, which stays within about ±0.3, up to full scale.
const GAIN: f32 = 3.5;

/// Noise that falls by 6 dB per octave, a deep sound like a far waterfall. It is the sound
/// played without a file, at full scale, without end.
pub struct BrownNoise {
    random: u32,
    walk: f32,
}

impl BrownNoise {
    pub fn new() -> Self {
        Self { random: 0x9e37_79b9, walk: 0.0 }
    }

    /// From -1 to 1, evenly spread: xorshift.
    fn white(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        self.random as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

impl Default for BrownNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for BrownNoise {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        self.walk = (self.walk + STEP * self.white()) / (1.0 + STEP);
        Some(((self.walk * GAIN).clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_SECOND: usize = 44_100;

    #[test]
    fn it_is_loud_and_never_ends() {
        let samples: Vec<i16> = BrownNoise::new().skip(10 * A_SECOND).take(A_SECOND).collect();
        assert_eq!(samples.len(), A_SECOND);
        let peak = samples.iter().map(|sample| sample.unsigned_abs()).max().unwrap();
        assert!(peak > u16::MAX / 8, "{peak}");
    }

    #[test]
    /// White noise moves further than its level, about 4/3 of it.
    fn it_is_deep_each_sample_close_to_the_last() {
        let samples: Vec<i32> = BrownNoise::new().take(A_SECOND).map(i32::from).collect();
        let mean_step = samples.windows(2).map(|pair| (pair[1] - pair[0]).abs()).sum::<i32>() / A_SECOND as i32;
        let mean_level = samples.iter().map(|sample| sample.abs()).sum::<i32>() / A_SECOND as i32;
        assert!(mean_step * 3 < mean_level, "steps of {mean_step} for a level of {mean_level}");
    }
}
