use core::num::NonZeroU8;

#[derive(Clone)]
pub struct Ch1Sweep {
    nr10: u8,
    pace_countdown: NonZeroU8,
    period_value: u16,
    // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
    // The “enabled flag” is set if either the sweep pace or individual step are non-zero, cleared otherwise.
    is_enabled: bool,
}

// according to blargg "Timer treats period 0 as 8"
const DEFAULT_PACE_COUTDOWN: NonZeroU8 = NonZeroU8::new(8).unwrap();

impl Default for Ch1Sweep {
    fn default() -> Self {
        Self {
            nr10: 0,
            pace_countdown: DEFAULT_PACE_COUTDOWN,
            period_value: 0,
            is_enabled: false,
        }
    }
}

impl Ch1Sweep {
    pub fn set_nr10(&mut self, value: u8) {
        self.nr10 = value;
    }
    pub fn get_nr10(&self) -> u8 {
        self.nr10
    }
    fn is_decreasing(&self) -> bool {
        self.nr10 & 0x08 != 0
    }
    // 3 bits
    fn individual_step(&self) -> u8 {
        self.nr10 & 0x07
    }

    fn pace(&self) -> u8 {
        (self.nr10 >> 4) & 0x07
    }

    // None -> overflow
    fn compute_next_value_and_check_overflow(&self) -> Option<u16> {
        if self.is_decreasing() {
            return Some(self.period_value - (self.period_value >> self.individual_step()));
        }

        let new_period = self.period_value + (self.period_value >> self.individual_step());

        log::info!(
            "0x{:04x} + 0x{:04x} = 0x{new_period:04x}",
            self.period_value,
            self.period_value >> self.individual_step(),
        );

        if new_period > 0x7ff {
            return None;
        }

        Some(new_period)
    }
}

pub trait Sweep {
    // returns new period value
    #[must_use]
    fn trigger(&mut self, period: u16) -> bool;
    // is channel still enable, new period value
    #[must_use]
    fn tick(&mut self) -> (bool, Option<u16>);
    #[must_use]
    fn get_period_value(&self) -> Option<u16>;
}

impl Sweep for Ch1Sweep {
    fn trigger(&mut self, period: u16) -> bool {
        self.period_value = period;
        self.pace_countdown = NonZeroU8::new(self.pace()).unwrap_or(DEFAULT_PACE_COUTDOWN);
        self.is_enabled = self.pace() != 0 || self.individual_step() != 0;
        log::info!("sweep trigger {}", self.is_enabled);
        // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
        // Citation: If the individual step is non-zero, frequency calculation and overflow check are performed immediately.
        self.individual_step() == 0 || self.compute_next_value_and_check_overflow().is_some()
    }

    // Returns channel on/off
    fn tick(&mut self) -> (bool, Option<u16>) {
        if !self.is_enabled {
            return (true, None);
        }

        // thanks gameroy for the order of steps
        if let Some(new_countdown) = NonZeroU8::new(self.pace_countdown.get() - 1) {
            self.pace_countdown = new_countdown;
            return (true, None);
        }

        // https://gbdev.io/pandocs/Audio_Registers.html#ff10--nr10-channel-1-sweep
        // Citation: Note that the value written to this field is not re-read by the hardware until a sweep iteration completes
        self.pace_countdown = NonZeroU8::new(self.pace()).unwrap_or(DEFAULT_PACE_COUTDOWN);

        if self.pace() == 0 {
            log::info!("sweep tick discarded");
            return (true, None);
        }

        // https://gbdev.io/pandocs/Audio_Registers.html#ff10--nr10-channel-1-sweep
        // Citation: In addition mode, if the period value would overflow (i.e. Lt+1 is
        // strictly more than $7FF), the channel is turned off instead. This occurs even
        // if sweep iterations are disabled by the pace being 0.
        let Some(new_period_value) = self.compute_next_value_and_check_overflow() else {
            return (false, None);
        };

        // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
        // Citation: If the new frequency is 2047 or less and **the individual step is not zero**,
        // this new frequency is written back to the “shadow register”
        if self.individual_step() == 0 {
            return (true, None);
        }

        self.period_value = new_period_value;

        // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
        // Citation: then frequency calculation and overflow check are run again immediately
        // using this new value, but this second new frequency is not written back
        (
            self.compute_next_value_and_check_overflow().is_some(),
            Some(new_period_value),
        )
    }

    fn get_period_value(&self) -> Option<u16> {
        if self.pace() == 0 && self.individual_step() == 0 {
            None
        } else {
            Some(self.period_value)
        }
    }
}

impl Sweep for () {
    fn trigger(&mut self, _: u16) -> bool {
        true
    }

    fn tick(&mut self) -> (bool, Option<u16>) {
        (true, None)
    }

    fn get_period_value(&self) -> Option<u16> {
        None
    }
}
