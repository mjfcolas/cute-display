use esp_idf_svc::hal::delay::{FreeRtos, BLOCK};
use esp_idf_svc::hal::gpio::{AnyIOPin, InputPin, Output, OutputPin, PinDriver};
use esp_idf_svc::hal::i2s::config::{DataBitWidth, StdConfig};
use esp_idf_svc::hal::i2s::{I2s, I2sDriver, I2sTx};
use hal::audio::Speaker;
use hal::Fault;

use crate::or_fault::OrFault;

const SAMPLE_RATE_HZ: u32 = 44_100;
const FRAMES_PER_WRITE: usize = 441;
const AMPLIFIER_WAKE_MS: u32 = 60;
/// Flushes the DMA ring before the amplifier goes off, so its last buffer is not looped.
const TRAILING_SILENCE_WRITES: usize = 6;

/// An I2S stereo stream into an amplifier with an enable pin. Mono samples go to both
/// channels.
pub struct I2sSpeaker {
    i2s: I2sDriver<'static, I2sTx>,
    amplifier: PinDriver<'static, Output>,
}

impl I2sSpeaker {
    pub fn new(
        i2s: impl I2s + 'static,
        bit_clock: impl InputPin + OutputPin + 'static,
        data_out: impl OutputPin + 'static,
        word_select: impl InputPin + OutputPin + 'static,
        amplifier_enable: impl OutputPin + 'static,
    ) -> Result<Self, Fault> {
        let config = StdConfig::philips(SAMPLE_RATE_HZ, DataBitWidth::Bits16);
        let i2s = I2sDriver::new_std_tx(i2s, &config, bit_clock, data_out, AnyIOPin::none(), word_select)
            .or_fault("I2S")?;
        let mut amplifier = PinDriver::output(amplifier_enable).or_fault("amplifier enable")?;
        amplifier.set_low().or_fault("amplifier enable")?;
        Ok(Self { i2s, amplifier })
    }

    fn stream(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
        self.i2s.tx_enable().or_fault("I2S enable")?;
        self.amplifier.set_high().or_fault("amplifier enable")?;
        FreeRtos::delay_ms(AMPLIFIER_WAKE_MS);
        let mut silence = 0;
        let mut bytes = [0u8; FRAMES_PER_WRITE * 4];
        while silence < TRAILING_SILENCE_WRITES {
            let mut frames = 0;
            for frame in bytes.chunks_exact_mut(4) {
                let Some(sample) = samples.next() else { break };
                let [lo, hi] = sample.to_le_bytes();
                frame.copy_from_slice(&[lo, hi, lo, hi]);
                frames += 1;
            }
            if frames == 0 {
                bytes.fill(0);
                frames = FRAMES_PER_WRITE;
                silence += 1;
            }
            self.i2s.write_all(bytes.get(..frames * 4).unwrap_or(&[]), BLOCK).or_fault("I2S write")?;
        }
        Ok(())
    }
}

impl Speaker for I2sSpeaker {
    fn sample_rate_hz(&self) -> u32 {
        SAMPLE_RATE_HZ
    }

    fn play(&mut self, samples: &mut dyn Iterator<Item = i16>) -> Result<(), Fault> {
        let played = self.stream(samples);
        let off = self.amplifier.set_low().or_fault("amplifier enable");
        let disabled = self.i2s.tx_disable().or_fault("I2S disable");
        played.and(off).and(disabled)
    }
}
