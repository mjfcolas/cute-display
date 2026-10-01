use domain::files::Files;

use crate::mp3::Mp3Samples;
use crate::wav::WavSamples;

/// "RIFF", the size, "WAVE".
const RIFF_HEADER_BYTES: usize = 12;

/// A file's samples at its own rate, its channels mixed, at full scale, round and round;
/// matched rather than boxed, as a call through a vtable for every sample counts on the
/// speaker's thread.
enum Decoder {
    Mp3(Mp3Samples),
    Wav(WavSamples),
}

/// A sound file, MP3 or WAV by what it holds, round and round, in mono at the speaker's
/// rate and at full scale, decoded as it plays. Ends when the file cannot be read or holds
/// no sound it can play.
pub struct Recording {
    decoded: Option<Decoder>,
    files: Option<Box<dyn Files>>,
    name: String,
    sample_rate_hz: u32,
    /// Where the next sample falls between `before` and `after`, two of the file's.
    phase: f32,
    file_rate_hz: u32,
    /// How far the phase moves each sample: the file's rate over the speaker's.
    step: f32,
    before: f32,
    after: f32,
}

impl Recording {
    /// Reads nothing yet: the file is read as it plays, where it plays.
    pub fn new(files: Box<dyn Files>, name: String, sample_rate_hz: u32) -> Self {
        Self { decoded: None, files: Some(files), name, sample_rate_hz, phase: 1.0, file_rate_hz: 0, step: 1.0, before: 0.0, after: 0.0 }
    }

    /// Reads the file's start once, to know how it is decoded.
    fn open(&mut self) {
        if let Some(files) = self.files.take() {
            self.decoded = match files.read_bytes(&self.name, 0, RIFF_HEADER_BYTES) {
                Ok(Some(start)) if is_wav(&start) => Some(Decoder::Wav(WavSamples::new(files, self.name.clone()))),
                Ok(_) => Some(Decoder::Mp3(Mp3Samples::new(files, self.name.clone()))),
                Err(unavailable) => {
                    log::warn!("sound: {}: {unavailable}", self.name);
                    None
                }
            };
        }
    }

    /// Runs on the speaker's thread for every sample of the file: only a read of the card
    /// allocates.
    fn next_of_the_file(&mut self) -> Option<f32> {
        let decoded = self.decoded.as_mut()?;
        let next = match decoded {
            Decoder::Mp3(mp3) => mp3.next_sample().map(|sample| (sample, mp3.sample_rate_hz())),
            Decoder::Wav(wav) => wav.next_sample().map(|sample| (sample, wav.sample_rate_hz())),
        };
        let Some((sample, file_rate_hz)) = next else {
            self.decoded = None;
            return None;
        };
        if file_rate_hz != self.file_rate_hz {
            self.file_rate_hz = file_rate_hz;
            self.step = file_rate_hz as f32 / self.sample_rate_hz.max(1) as f32;
        }
        Some(sample)
    }
}

impl Iterator for Recording {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        self.open();
        while self.phase >= 1.0 {
            self.before = self.after;
            self.after = self.next_of_the_file()?;
            self.phase -= 1.0;
        }
        let sample = self.before + (self.after - self.before) * self.phase;
        self.phase += self.step;
        Some((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
    }
}

fn is_wav(start: &[u8]) -> bool {
    matches!(start.get(..RIFF_HEADER_BYTES), Some([b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'A', b'V', b'E']))
}
