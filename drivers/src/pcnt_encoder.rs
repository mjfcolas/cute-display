//! A quadrature encoder counted by the PCNT peripheral, so no detent is missed however
//! seldom it is read.

use core::time::Duration;

use esp_idf_svc::hal::gpio::InputPin;
use esp_idf_svc::hal::pcnt::config::{ChannelConfig, ChannelEdgeAction, ChannelLevelAction, GlitchFilterConfig, UnitConfig};
use esp_idf_svc::hal::pcnt::PcntUnitDriver;
use hal::input::RotaryEncoder;
use hal::Fault;

use crate::or_fault::OrFault;

/// Both edges of A are counted, and the wheel clicks once per quadrature cycle.
const COUNTS_PER_DETENT: i32 = 2;

pub struct PcntEncoder {
    unit: PcntUnitDriver<'static>,
    counted: i32,
}

impl PcntEncoder {
    pub fn new(a: impl InputPin + 'static, b: impl InputPin + 'static) -> Result<Self, Fault> {
        let mut unit = PcntUnitDriver::new(&UnitConfig::default()).or_fault("PCNT unit")?;
        unit.set_glitch_filter(Some(&GlitchFilterConfig { max_glitch: Duration::from_micros(1), ..Default::default() }))
            .or_fault("PCNT glitch filter")?;
        unit.add_channel(Some(a), Some(b), &ChannelConfig::default())
            .or_fault("PCNT channel")?
            .set_edge_action(ChannelEdgeAction::Increase, ChannelEdgeAction::Decrease)
            .or_fault("PCNT edge action")?
            .set_level_action(ChannelLevelAction::Keep, ChannelLevelAction::Inverse)
            .or_fault("PCNT level action")?;
        unit.enable().or_fault("PCNT enable")?;
        unit.start().or_fault("PCNT start")?;
        Ok(Self { unit, counted: 0 })
    }
}

impl RotaryEncoder for PcntEncoder {
    fn take_detents(&mut self) -> i32 {
        let Ok(count) = self.unit.get_count() else {
            return 0;
        };
        // A half-turned detent stays in the counter until it completes.
        let detents = count.saturating_sub(self.counted) / COUNTS_PER_DETENT;
        self.counted = self.counted.saturating_add(detents * COUNTS_PER_DETENT);
        detents
    }
}
