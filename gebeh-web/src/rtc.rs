use std::{cell::Cell, rc::Rc};

use gebeh_core::mbc::*;

#[derive(Default, Clone)]
pub struct NullRtc;

impl Rtc for NullRtc {
    fn get_clock_data(&mut self) -> RtcRegisters {
        Default::default()
    }

    fn set_clock_data(&mut self, _: RtcRegisters) {}

    fn deserialize(&mut self, _: &[u8]) {}

    fn serialize(&self, _: &mut [u8]) -> usize {
        0
    }
}

// web-time doesn't work in AudioWorklet according to https://github.com/daxpedda/web-time/issues/45
#[derive(Clone)]
pub struct InstantRtc {
    last_seen: u64,
    last_halt: Option<u64>,
    registers: RtcRegisters,
    now: Rc<Cell<u64>>,
}

impl InstantRtc {
    pub fn new(now: Rc<Cell<u64>>) -> Self {
        Self {
            last_seen: now.get(),
            last_halt: None,
            registers: Default::default(),
            now,
        }
    }
}

impl Rtc for InstantRtc {
    fn get_clock_data(&mut self) -> RtcRegisters {
        let now = self.last_halt.unwrap_or(self.now.get());
        let elapsed = now - self.last_seen;
        self.last_seen = now;
        let new_seconds = u64::from(self.registers.get_total_seconds()) + elapsed;
        let new_registers = RtcRegisters::from_seconds(
            u32::try_from(new_seconds % u64::from(MAX_RTC_SECONDS)).unwrap(),
            new_seconds > u64::from(MAX_RTC_SECONDS),
            (self.registers.upper_1bit_day_counter_carry_halt & 0b10) != 0,
        );
        self.registers = new_registers;
        new_registers
    }

    fn set_clock_data(&mut self, registers: RtcRegisters) {
        let now = self.now.get();
        self.registers = registers;
        self.last_seen = now;
        self.last_halt = (registers.upper_1bit_day_counter_carry_halt & 0b10 != 0).then_some(now)
    }

    // u64 seconds since epoch, u32 seconds of mbc3 clock data, big endian
    fn deserialize(&mut self, save: &[u8]) {
        let saved_system_seconds = u64::from_be_bytes(save[..8].try_into().unwrap());
        let saved_rtc_seconds = u32::from_be_bytes(save[8..12].try_into().unwrap());
        let new_seconds = u64::from(saved_rtc_seconds) + (self.now.get() - saved_system_seconds);
        self.last_seen = self.now.get();
        self.registers = RtcRegisters::from_seconds(
            u32::try_from(new_seconds % u64::from(MAX_RTC_SECONDS)).unwrap(),
            new_seconds > u64::from(MAX_RTC_SECONDS),
            (self.registers.upper_1bit_day_counter_carry_halt & 0b10) != 0,
        );
    }

    fn serialize(&self, buffer: &mut [u8]) -> usize {
        buffer[..8].copy_from_slice(&self.now.get().to_be_bytes());
        buffer[8..12].copy_from_slice(&self.registers.get_total_seconds().to_be_bytes());
        12
    }
}
