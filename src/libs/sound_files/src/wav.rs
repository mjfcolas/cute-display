use domain::files::Files;

const CHUNK_HEADER_BYTES: u64 = 8;
const FIRST_CHUNK: u64 = 12;
/// Past these, a file is taken for holding no samples rather than read on.
const MOST_CHUNKS_BEFORE_THE_SAMPLES: usize = 32;
const PCM: u16 = 1;
const EXTENSIBLE: u16 = 0xfffe;
const BYTES_PER_SAMPLE: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    channels: usize,
    sample_rate_hz: u32,
    samples_start: u64,
    samples_end: u64,
}

impl Layout {
    fn frame_bytes(&self) -> usize {
        self.channels * BYTES_PER_SAMPLE
    }
}

/// A WAV file's 16-bit PCM samples, its channels mixed, at its own rate, round and round.
pub(crate) struct WavSamples {
    files: Box<dyn Files>,
    name: String,
    layout: Option<Layout>,
    bytes: Vec<u8>,
    used: usize,
    read_to: u64,
    played_this_round: bool,
}

impl WavSamples {
    pub(crate) fn new(files: Box<dyn Files>, name: String) -> Self {
        Self { files, name, layout: None, bytes: Vec::new(), used: 0, read_to: 0, played_this_round: false }
    }

    fn layout(&mut self) -> Result<Layout, String> {
        match self.layout {
            Some(layout) => Ok(layout),
            None => {
                let layout = self.read_layout()?;
                self.layout = Some(layout);
                self.read_to = layout.samples_start;
                Ok(layout)
            }
        }
    }

    fn read_layout(&self) -> Result<Layout, String> {
        let mut format = None;
        let mut at = FIRST_CHUNK;
        for _ in 0..MOST_CHUNKS_BEFORE_THE_SAMPLES {
            let header = self.read_exactly(at, CHUNK_HEADER_BYTES as usize)?;
            let (id, size) = header.split_at(4);
            let size = u64::from(u32::from_le_bytes(size.try_into().map_err(|_| "a chunk cut short")?));
            let body = at + CHUNK_HEADER_BYTES;
            match id {
                b"fmt " => format = Some(decode_format(&self.read_exactly(body, size.min(40) as usize)?)?),
                b"data" => {
                    let (channels, sample_rate_hz) = format.ok_or("samples before their format")?;
                    return Ok(Layout { channels, sample_rate_hz, samples_start: body, samples_end: body + size });
                }
                _ => {}
            }
            at = body + size + size % 2;
        }
        Err("no samples in it".into())
    }

    fn read_exactly(&self, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        let bytes = self.files.read_bytes(&self.name, offset, length).map_err(|unavailable| unavailable.0)?.ok_or("not there")?;
        if bytes.len() < length {
            return Err("cut short".into());
        }
        Ok(bytes)
    }

    fn read_on_or_round_again(&mut self, layout: Layout) -> Result<(), String> {
        self.bytes.drain(..self.used);
        self.used = 0;
        let wanted = usize::try_from(layout.samples_end.saturating_sub(self.read_to)).unwrap_or(usize::MAX).min(crate::READ_BYTES);
        let read = match wanted {
            0 => Vec::new(),
            _ => self.files.read_bytes(&self.name, self.read_to, wanted).map_err(|unavailable| unavailable.0)?.ok_or("not there")?,
        };
        if read.is_empty() {
            if !self.played_this_round {
                return Err("no samples in it".into());
            }
            self.bytes.clear();
            self.read_to = layout.samples_start;
            self.played_this_round = false;
            return Ok(());
        }
        self.read_to += read.len() as u64;
        self.bytes.extend_from_slice(&read);
        Ok(())
    }
}

impl WavSamples {
    /// From -1 to 1; `None` once the file cannot be read or holds no samples it can play.
    pub(crate) fn next_sample(&mut self) -> Option<f32> {
        match self.next_frame() {
            Ok(frame) => Some(frame),
            Err(reason) => {
                log::warn!("sound: {}: {reason}", self.name);
                None
            }
        }
    }

    pub(crate) fn sample_rate_hz(&self) -> u32 {
        self.layout.map_or(0, |layout| layout.sample_rate_hz)
    }

    /// Its channels mixed.
    #[inline]
    fn next_frame(&mut self) -> Result<f32, String> {
        let layout = self.layout()?;
        let frame_bytes = layout.frame_bytes();
        while self.bytes.len() - self.used < frame_bytes {
            self.read_on_or_round_again(layout)?;
        }
        let frame = self.bytes.get(self.used..self.used + frame_bytes).ok_or("a frame cut short")?;
        let (samples, _) = frame.as_chunks::<BYTES_PER_SAMPLE>();
        let sum: i32 = samples.iter().map(|&sample| i32::from(i16::from_le_bytes(sample))).sum();
        self.used += frame_bytes;
        self.played_this_round = true;
        Ok(sum as f32 / (layout.channels as f32 * f32::from(i16::MAX)))
    }
}

/// The channels and the rate of a `fmt ` chunk of 16-bit PCM.
fn decode_format(chunk: &[u8]) -> Result<(usize, u32), String> {
    let field = |at: usize| chunk.get(at..at + 2).and_then(|bytes| bytes.try_into().ok()).map(u16::from_le_bytes);
    let (Some(tag), Some(channels), Some(bits)) = (field(0), field(2), field(14)) else {
        return Err("a format cut short".into());
    };
    let rate = chunk.get(4..8).and_then(|bytes| bytes.try_into().ok()).map_or(0, u32::from_le_bytes);
    let sub_format = field(24);
    match (tag, sub_format, bits, channels, rate) {
        (PCM, _, 16, 1.., 1..) | (EXTENSIBLE, Some(PCM), 16, 1.., 1..) => Ok((usize::from(channels), rate)),
        (PCM, ..) | (EXTENSIBLE, Some(PCM), ..) => Err(format!("{bits}-bit samples, not 16")),
        _ => Err("samples not in PCM".into()),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use core::f32::consts::TAU;

    use domain_testing::files::FakeFiles;

    use super::*;
    use crate::Recording;

    const RATE: u32 = 44_100;

    /// A WAV of `frames`, each a sample per channel, a `JUNK` chunk before its format as
    /// Habity's have.
    pub(crate) fn wav(channels: u16, rate: u32, bits: u16, frames: &[Vec<i16>]) -> Vec<u8> {
        let samples: Vec<u8> = frames.iter().flatten().flat_map(|sample| sample.to_le_bytes()).collect();
        let mut format = Vec::new();
        format.extend_from_slice(&PCM.to_le_bytes());
        format.extend_from_slice(&channels.to_le_bytes());
        format.extend_from_slice(&rate.to_le_bytes());
        format.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        format.extend_from_slice(&(channels * 2).to_le_bytes());
        format.extend_from_slice(&bits.to_le_bytes());
        let mut chunks = Vec::new();
        for (id, body) in [(b"JUNK", vec![0; 5]), (b"fmt ", format), (b"data", samples)] {
            chunks.extend_from_slice(id);
            chunks.extend_from_slice(&(body.len() as u32).to_le_bytes());
            let padding = body.len() % 2;
            chunks.extend(body);
            chunks.extend(std::iter::repeat_n(0, padding));
        }
        let mut file = b"RIFF".to_vec();
        file.extend_from_slice(&(chunks.len() as u32 + 4).to_le_bytes());
        file.extend_from_slice(b"WAVE");
        file.extend(chunks);
        file
    }

    pub(crate) fn sine(hz: f32, rate: u32, seconds: f32, peak: i16) -> Vec<i16> {
        (0..(rate as f32 * seconds) as u32).map(|n| ((n as f32 * hz * TAU / rate as f32).sin() * f32::from(peak)) as i16).collect()
    }

    fn recording_of(contents: &[u8]) -> Recording {
        Recording::new(Box::new(FakeFiles::with("sounds/tone.wav", contents)), "sounds/tone.wav".into(), RATE)
    }

    fn frequency_hz(samples: &[i16]) -> f32 {
        let rising = samples.windows(2).filter(|pair| pair[0] < 0 && pair[1] >= 0).count();
        rising as f32 * RATE as f32 / samples.len() as f32
    }

    #[test]
    fn a_wav_plays_its_samples_its_channels_mixed() {
        let frames = [vec![1000, 3000], vec![-2000, -4000], vec![0, 100]];
        let played: Vec<i16> = recording_of(&wav(2, RATE, 16, &frames)).skip(1).take(3).collect();
        assert_eq!(played, [2000, -3000, 50]);
    }

    #[test]
    fn a_wav_comes_round_again_without_a_gap() {
        let frames = [vec![100], vec![200], vec![300]];
        let played: Vec<i16> = recording_of(&wav(1, RATE, 16, &frames)).skip(1).take(7).collect();
        assert_eq!(played, [100, 200, 300, 100, 200, 300, 100]);
    }

    #[test]
    fn a_wav_longer_than_a_read_plays_whole() {
        let tone = sine(440.0, RATE, 0.5, 8000);
        let frames: Vec<Vec<i16>> = tone.iter().map(|&sample| vec![sample, sample]).collect();
        let played: Vec<i16> = recording_of(&wav(2, RATE, 16, &frames)).skip(1).take(tone.len()).collect();
        assert_eq!(played, tone);
    }

    #[test]
    fn a_wav_at_another_rate_plays_at_the_speakers() {
        let frames: Vec<Vec<i16>> = sine(1000.0, 22_050, 0.5, 8000).into_iter().map(|sample| vec![sample]).collect();
        let samples: Vec<i16> = recording_of(&wav(1, 22_050, 16, &frames)).skip(1000).take(RATE as usize / 5).collect();
        assert!((frequency_hz(&samples) - 1000.0).abs() < 20.0, "{} Hz", frequency_hz(&samples));
    }

    #[test]
    fn a_wav_it_cannot_play_ends() {
        let frames = [vec![100], vec![200]];
        assert_eq!(recording_of(&wav(1, RATE, 8, &frames)).next(), None, "8-bit");
        assert_eq!(recording_of(&wav(1, RATE, 16, &[])).count(), 0, "no samples");
        let mut cut = wav(1, RATE, 16, &frames);
        cut.truncate(30);
        assert_eq!(recording_of(&cut).count(), 0, "cut in its chunks");
    }

    #[test]
    fn the_format_says_its_channels_and_rate() {
        assert_eq!(decode_format(wav(2, RATE, 16, &[]).get(34..50).unwrap()), Ok((2, RATE)));
        assert!(decode_format(&[1, 0]).is_err());
    }
}
