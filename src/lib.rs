use std::{ops::Deref, time::Instant};

use gebeh_core::mbc::*;

pub trait CloneMbc<'a>: Mbc {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a>;
}

impl<'a, T: Mbc + Clone + 'a> CloneMbc<'a> for T {
    fn clone_boxed(&self) -> Box<dyn CloneMbc<'a> + 'a> {
        Box::new(self.clone())
    }
}

pub fn get_mbc<'a, T: Deref<Target = [u8]> + Clone + 'a>(
    rom: T,
) -> Option<Box<dyn CloneMbc<'a> + 'a>> {
    let mbc: Box<dyn CloneMbc<'a>> =
        match CartridgeType::try_from(rom.get(0x147).copied().unwrap_or(0)).ok()? {
            CartridgeType::RomOnly => Box::new(rom),
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram => Box::new(Mbc1::new(rom)),
            CartridgeType::Mbc3RamBattery => Box::new(Mbc3::new(rom, InstantRtc::new())),
            CartridgeType::Mbc5RamBattery => Box::new(Mbc5::new(rom)),
        };
    Some(mbc)
}

#[derive(Clone)]
struct InstantRtc {
    last_seen: Instant,
    last_halt: Option<Instant>,
}

impl InstantRtc {
    fn new() -> Self {
        Self {
            last_seen: Instant::now(),
            last_halt: None,
        }
    }
}

impl Rtc for InstantRtc {
    fn get_clock_data(&mut self, current: RtcRegisters) -> RtcRegisters {
        let now = self.last_halt.unwrap_or_else(Instant::now);
        let elapsed = now - self.last_seen;
        self.last_seen = now;
        let new_seconds = u64::from(current.get_total_seconds()) + elapsed.as_secs();
        let has_carried = new_seconds > u64::from(MAX_RTC_SECONDS);
        let new_seconds = new_seconds % u64::from(MAX_RTC_SECONDS);
        let new_days = u16::try_from((new_seconds / (3600 * 24)) % 512).unwrap();
        RtcRegisters {
            seconds: u8::try_from(new_seconds % 60).unwrap(),
            minutes: u8::try_from((new_seconds / 60) % 60).unwrap(),
            hours: u8::try_from((new_seconds / 3600) % 24).unwrap(),
            lower_8bits_day_counter: new_days as u8,
            upper_1bit_day_counter_carry_halt: ((new_days >> 1) as u8) & 0x80
                | (current.upper_1bit_day_counter_carry_halt & 0b10)
                | has_carried as u8,
        }
    }

    fn set_clock_data(&mut self, register: RtcRegisters) {
        let now = Instant::now();
        self.last_seen = now;
        self.last_halt = (register.upper_1bit_day_counter_carry_halt & 0b10 != 0).then_some(now)
    }
}
