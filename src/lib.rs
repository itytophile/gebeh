use std::{
    ops::Deref,
    time::{Instant, UNIX_EPOCH},
};

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
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
                Box::new(Mbc1::new(rom))
            }
            CartridgeType::Mbc3RamBattery => Box::new(Mbc3::new(rom, InstantRtc::new())),
            CartridgeType::Mbc5RamBattery => Box::new(Mbc5::new(rom)),
        };
    Some(mbc)
}

#[derive(Clone)]
struct InstantRtc {
    last_seen: Instant,
    last_halt: Option<Instant>,
    registers: RtcRegisters,
}

impl InstantRtc {
    fn new() -> Self {
        Self {
            last_seen: Instant::now(),
            last_halt: None,
            registers: Default::default(),
        }
    }
}

impl Rtc for InstantRtc {
    fn get_clock_data(&mut self) -> RtcRegisters {
        let now = self.last_halt.unwrap_or_else(Instant::now);
        let elapsed = now - self.last_seen;
        self.last_seen = now;
        let new_seconds = u64::from(self.registers.get_total_seconds()) + elapsed.as_secs();
        let new_registers = RtcRegisters::from_seconds(
            u32::try_from(new_seconds % u64::from(MAX_RTC_SECONDS)).unwrap(),
            new_seconds > u64::from(MAX_RTC_SECONDS),
            (self.registers.upper_1bit_day_counter_carry_halt & 0b10) != 0,
        );
        self.registers = new_registers;
        new_registers
    }

    fn set_clock_data(&mut self, registers: RtcRegisters) {
        let now = Instant::now();
        self.registers = registers;
        self.last_seen = now;
        self.last_halt = (registers.upper_1bit_day_counter_carry_halt & 0b10 != 0).then_some(now)
    }

    // u64 seconds since epoch, u32 seconds of mbc3 clock data, big endian
    fn deserialize(&mut self, save: &[u8]) {
        let saved_system_seconds = u64::from_be_bytes(save[..8].try_into().unwrap());
        let saved_rtc_seconds = u32::from_be_bytes(save[8..12].try_into().unwrap());
        let new_seconds = u64::from(saved_rtc_seconds)
            + (UNIX_EPOCH.elapsed().unwrap().as_secs() - saved_system_seconds);
        // yes it's not perfect to manipulate Instant and SystemTime at the same time but who cares
        self.last_seen = Instant::now();
        self.registers = RtcRegisters::from_seconds(
            u32::try_from(new_seconds % u64::from(MAX_RTC_SECONDS)).unwrap(),
            new_seconds > u64::from(MAX_RTC_SECONDS),
            (self.registers.upper_1bit_day_counter_carry_halt & 0b10) != 0,
        );
    }

    fn serialize(&self, buffer: &mut [u8]) -> usize {
        buffer[..8].copy_from_slice(&UNIX_EPOCH.elapsed().unwrap().as_secs().to_be_bytes());
        buffer[8..12].copy_from_slice(&self.registers.get_total_seconds().to_be_bytes());
        12
    }
}
