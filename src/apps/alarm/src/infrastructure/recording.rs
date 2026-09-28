use domain::files::Files;
use mbop3::{Decoder, MAX_SAMPLES_PER_FRAME};

/// What the decoder wants ahead of it to decode a frame whole.
const AHEAD_BYTES: usize = 16 * 1024;
/// Reading the card stops the samples meanwhile, and the speaker holds about 30 ms of them:
/// 16 KB took 16 ms on the device, 4 KB leave room for decoding a frame beside.
const READ_BYTES: usize = 4 * 1024;
const ID3_HEADER_BYTES: usize = 10;

/// An MP3 file of the alarm's, round and round, in mono at the speaker's rate and at full
/// scale, decoded as it plays. Ends when the file cannot be read or holds no MP3.
pub struct Recording {
    samples: Mp3Samples,
    sample_rate_hz: u32,
    /// Where the next sample falls between `before` and `after`, two of the file's.
    phase: f32,
    before: f32,
    after: f32,
}

impl Recording {
    /// Reads nothing yet: the file is read as it plays, where it plays.
    pub fn new(files: Box<dyn Files>, name: String, sample_rate_hz: u32) -> Self {
        Self { samples: Mp3Samples::new(files, name), sample_rate_hz, phase: 1.0, before: 0.0, after: 0.0 }
    }
}

impl Iterator for Recording {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        while self.phase >= 1.0 {
            self.before = self.after;
            self.after = self.samples.next()?;
            self.phase -= 1.0;
        }
        let sample = self.before + (self.after - self.before) * self.phase;
        self.phase += self.samples.frame.sample_rate_hz as f32 / self.sample_rate_hz.max(1) as f32;
        Some((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
    }
}

#[derive(Clone, Copy, Default)]
struct FrameFormat {
    samples: usize,
    channels: usize,
    sample_rate_hz: u32,
}

/// The file's samples, its channels mixed, at its own rate.
struct Mp3Samples {
    files: Box<dyn Files>,
    name: String,
    decoder: Box<Decoder>,
    /// Where the MP3 starts, past its tags; known once the file's start is read.
    start: Option<u64>,
    mp3: Vec<u8>,
    used: usize,
    read_to: u64,
    read_whole: bool,
    decoded_this_round: bool,
    pcm: Vec<f32>,
    frame: FrameFormat,
    next_sample: usize,
}

impl Mp3Samples {
    fn new(files: Box<dyn Files>, name: String) -> Self {
        Self {
            files,
            name,
            decoder: Decoder::new_boxed(),
            start: None,
            mp3: Vec::new(),
            used: 0,
            read_to: 0,
            read_whole: false,
            decoded_this_round: false,
            pcm: vec![0.0; MAX_SAMPLES_PER_FRAME],
            frame: FrameFormat::default(),
            next_sample: 0,
        }
    }

    fn next(&mut self) -> Option<f32> {
        while self.next_sample >= self.frame.samples {
            if let Err(reason) = self.decode_frame() {
                log::warn!("alarm: {}: {reason}", self.name);
                return None;
            }
        }
        let at = self.next_sample * self.frame.channels;
        let channels = self.pcm.get(at..at + self.frame.channels)?;
        self.next_sample += 1;
        Some(channels.iter().sum::<f32>() / self.frame.channels as f32)
    }

    fn decode_frame(&mut self) -> Result<(), String> {
        loop {
            while self.unused() < AHEAD_BYTES && !self.read_whole {
                self.read_more()?;
            }
            if self.unused() == 0 {
                if !self.decoded_this_round {
                    return Err("no MP3 in it".into());
                }
                self.rewind();
                continue;
            }
            let pcm = self.pcm.as_mut_slice().try_into().ok();
            let (samples, frame) = self.decoder.decode_frame(self.mp3.get(self.used..).unwrap_or_default(), pcm);
            let used = usize::try_from(frame.frame_bytes).unwrap_or(0);
            self.used = (self.used + used).min(self.mp3.len());
            match (samples, usize::try_from(frame.channels), u32::try_from(frame.hz)) {
                (1.., Ok(channels @ 1..), Ok(sample_rate_hz)) => {
                    self.frame = FrameFormat { samples, channels, sample_rate_hz };
                    self.next_sample = 0;
                    self.decoded_this_round = true;
                    return Ok(());
                }
                _ if used == 0 && self.read_whole => self.used = self.mp3.len(),
                _ if used == 0 => self.read_more()?,
                _ => {}
            }
        }
    }

    fn unused(&self) -> usize {
        self.mp3.len() - self.used
    }

    fn read_more(&mut self) -> Result<(), String> {
        self.mp3.drain(..self.used);
        self.used = 0;
        let bytes = self.files.read_bytes(&self.name, self.read_to, READ_BYTES).map_err(|unavailable| unavailable.0)?;
        let bytes = bytes.ok_or("not there")?;
        self.read_whole = bytes.len() < READ_BYTES;
        self.read_to += bytes.len() as u64;
        self.mp3.extend_from_slice(&bytes);
        if self.start.is_none() {
            let start = id3_tag_length(&bytes);
            self.start = Some(start);
            if start > self.read_to {
                self.rewind();
                return self.read_more();
            } else {
                self.used = usize::try_from(start).unwrap_or(0);
            }
        }
        Ok(())
    }

    /// Back to the start of the MP3, afresh.
    fn rewind(&mut self) {
        self.decoder = Decoder::new_boxed();
        self.mp3.clear();
        self.used = 0;
        self.read_to = self.start.unwrap_or(0);
        self.read_whole = false;
        self.decoded_this_round = false;
    }
}

/// An ID3v2 tag's length, header and footer included; 0 without a tag.
fn id3_tag_length(start: &[u8]) -> u64 {
    let Some(&[b'I', b'D', b'3', _, _, flags, s0, s1, s2, s3]) = start.get(..ID3_HEADER_BYTES) else {
        return 0;
    };
    let size = [s0, s1, s2, s3].iter().fold(0u64, |size, &byte| size << 7 | u64::from(byte & 0x7f));
    let footer = if flags & 0x10 != 0 { ID3_HEADER_BYTES as u64 } else { 0 };
    ID3_HEADER_BYTES as u64 + size + footer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory_files::MemoryFiles;

    const STEREO_440_HZ: &[u8] = include_bytes!("../../testdata/a440-stereo-44100.mp3");
    const MONO_1000_HZ_AT_22050: &[u8] = include_bytes!("../../testdata/a1000-mono-22050.mp3");
    const RATE: u32 = 44_100;
    /// The fixture's loudest sample, as ffmpeg decodes it.
    const SINE_PEAK: i16 = 2752;

    fn recording_of(contents: Option<&[u8]>) -> Recording {
        let files = MemoryFiles::default();
        if let Some(contents) = contents {
            files.put("ringtones/tone.mp3", contents);
        }
        Recording::new(Box::new(files), "ringtones/tone.mp3".into(), RATE)
    }

    /// How many times a second the samples cross zero going up.
    fn frequency_hz(samples: &[i16]) -> f32 {
        let rising = samples.windows(2).filter(|pair| pair[0] < 0 && pair[1] >= 0).count();
        rising as f32 * RATE as f32 / samples.len() as f32
    }

    #[test]
    fn an_mp3_plays_as_the_tone_it_holds_as_loud() {
        let samples: Vec<i16> = recording_of(Some(STEREO_440_HZ)).skip(4_000).take(RATE as usize / 5).collect();
        assert!((frequency_hz(&samples) - 440.0).abs() < 10.0, "{} Hz", frequency_hz(&samples));
        let peak = samples.iter().map(|s| s.abs()).max().unwrap();
        assert!((peak - SINE_PEAK).abs() < SINE_PEAK / 50, "{peak}");
    }

    #[test]
    fn an_mp3_at_another_rate_plays_at_the_speakers() {
        let samples: Vec<i16> = recording_of(Some(MONO_1000_HZ_AT_22050)).skip(8_000).take(RATE as usize / 5).collect();
        assert!((frequency_hz(&samples) - 1000.0).abs() < 20.0, "{} Hz", frequency_hz(&samples));
    }

    #[test]
    fn an_mp3_comes_round_again() {
        let half_a_second = RATE as usize / 2;
        let played = recording_of(Some(STEREO_440_HZ)).take(3 * half_a_second).count();
        assert_eq!(played, 3 * half_a_second);
    }

    #[test]
    fn a_file_missing_or_holding_no_mp3_ends() {
        assert_eq!(recording_of(None).next(), None);
        assert_eq!(recording_of(Some(b"")).next(), None);
        assert_eq!(recording_of(Some(&[0; 40_000])).count(), 0);
    }

    #[test]
    fn a_tag_longer_than_a_read_is_skipped() {
        let mut tagged = b"ID3\x04\x00\x00\x00\x01\x00\x01".to_vec();
        tagged.resize(ID3_HEADER_BYTES + (1 << 14) + 1, 0);
        assert_eq!(id3_tag_length(&tagged), tagged.len() as u64);
        tagged.extend_from_slice(STEREO_440_HZ);
        let samples: Vec<i16> = recording_of(Some(&tagged)).skip(4_000).take(RATE as usize / 5).collect();
        assert!((frequency_hz(&samples) - 440.0).abs() < 10.0, "{} Hz", frequency_hz(&samples));
    }
}
