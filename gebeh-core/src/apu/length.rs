pub const MASK_8_BITS: u8 = 0xff;
pub const MASK_6_BITS: u8 = 0x3f;

#[derive(Clone, Default, Debug)]
pub struct Length<const MASK: u8> {
    is_enabled: bool,
    current_timer_value: Option<u8>, // None = overflowed
}

impl<const MASK: u8> Length<MASK> {
    // returns true if overflow
    #[must_use]
    pub fn set_is_enabled(
        &mut self,
        is_enabled: bool,
        ch: &'static str,
        extra_clock: bool,
    ) -> bool {
        let previous_is_length_enabled = self.is_enabled;
        self.is_enabled = is_enabled;
        if !previous_is_length_enabled && self.is_enabled {
            log::info!("{ch} length enabled!");
            // for this hack to work, the cpu must be executed after the apu (I suppose)
            // according to blargg "Enabling in first half of length period should clock length"
            if extra_clock {
                log::info!("extra tick");
                return self.tick();
            }
        }

        false
    }
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }
    pub fn set_initial_timer_length(&mut self, value: u8) {
        // according to blargg "Length can be reloaded at any time"
        self.current_timer_value = Some(MASK - (value & MASK));
    }

    pub fn trigger(&mut self, extra_clock: bool) {
        log::info!("trigger with extra = {}, {:?}", extra_clock, self);
        if self.current_timer_value.is_none() {
            // according to blargg "Trigger that un-freezes enabled length should clock it"
            self.current_timer_value = Some(MASK - (extra_clock && self.is_enabled) as u8);
        }
    }

    // returns true if overflow
    #[must_use]
    pub fn tick(&mut self) -> bool {
        let (Some(prout), true) = (self.current_timer_value, self.is_enabled) else {
            return false;
        };
        
        log::info!("tick length {prout}");

        self.current_timer_value = prout.checked_sub(1);

        if self.current_timer_value.is_none() {
            log::info!("length expired");
        }

        self.current_timer_value.is_none()
    }
}
