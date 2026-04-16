use crate::{mbc::*, state::*};
use core::ops::Deref;

#[derive(Clone, Copy)]
enum RtcSelect {
    Seconds,
    Minutes,
    Hours,
    Lower8bitsDayCounter,
    Upper1bitDayCounterCarryHalt,
}

pub const MAX_RTC_SECONDS: u32 = 511 * 24 * 60 * 60 + 23 * 60 * 60 + 59 * 60 + 59;

#[derive(Clone, Copy, Default, Debug)]
pub struct RtcRegisters {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub lower_8bits_day_counter: u8,
    pub upper_1bit_day_counter_carry_halt: u8,
}

impl RtcRegisters {
    pub fn get_total_seconds(&self) -> u32 {
        u32::from(self.seconds)
            + u32::from(self.minutes) * 60
            + u32::from(self.hours) * 60 * 60
            + u32::from(self.get_day_counter()) * 24 * 60 * 60
    }
    pub fn get_day_counter(&self) -> u16 {
        ((u16::from(self.upper_1bit_day_counter_carry_halt & 0x01)) << 8)
            | u16::from(self.lower_8bits_day_counter)
    }
    pub fn from_seconds(seconds: u32, carry: bool, halt: bool) -> Self {
        let new_days = u16::try_from((seconds / (3600 * 24)) % 512).unwrap();
        RtcRegisters {
            seconds: u8::try_from(seconds % 60).unwrap(),
            minutes: u8::try_from((seconds / 60) % 60).unwrap(),
            hours: u8::try_from((seconds / 3600) % 24).unwrap(),
            lower_8bits_day_counter: new_days as u8,
            upper_1bit_day_counter_carry_halt: ((new_days >> 8) as u8 & 0x01)
                | ((halt as u8) << 6)
                | ((carry as u8) << 7),
        }
    }
}

#[derive(Clone, Copy)]
enum RamRtcSelect {
    Ram(u8),
    Rtc(RtcSelect),
}

#[derive(Clone)]
pub struct Mbc3<T, U> {
    rom: T,
    rom_offset: usize,
    // 32 KiB
    ram: [u8; 0x8000],
    ram_rtc_select: RamRtcSelect,
    ram_enabled: bool,
    rom_bank_count: u8,
    rtc: U,
    rtc_registers: RtcRegisters,
    latch_reg: u8,
}

pub trait Rtc {
    fn get_clock_data(&mut self) -> RtcRegisters;
    fn set_clock_data(&mut self, register: RtcRegisters);
    fn deserialize(&mut self, save: &[u8]);
    fn serialize(&self, buffer: &mut [u8]) -> usize;
}

impl<T: Deref<Target = [u8]>, U> Mbc3<T, U> {
    pub fn new(rom: T, rtc: U) -> Self {
        Self {
            rom_bank_count: u8::try_from(get_factor_32_kib_rom(rom.deref())).unwrap() * 2,
            rom,
            rom_offset: usize::from(ROM_BANK_SIZE),
            ram_rtc_select: RamRtcSelect::Ram(0),
            ram: [0; 0x8000],
            ram_enabled: false,
            rtc,
            rtc_registers: Default::default(),
            // Citation: When writing $00, and then $01 to this register, the current time becomes latched into the RTC registers
            latch_reg: 2,
        }
    }

    pub fn set_rom_bank(&mut self, rom_bank: u16) {
        self.rom_offset = usize::from(rom_bank) * usize::from(ROM_BANK_SIZE);
    }
}

impl<T: Deref<Target = [u8]>, U: Rtc> Mbc for Mbc3<T, U> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => *self
                .rom
                .get(self.rom_offset + (index - 0x4000) as usize)
                .unwrap_or(&0x0),
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return 0xff;
                }
                use RtcSelect::*;
                match self.ram_rtc_select {
                    RamRtcSelect::Ram(bank) => {
                        self.ram[usize::from(u16::from(bank) * RAM_BANK_SIZE)
                            + (index - 0xa000) as usize]
                    }
                    RamRtcSelect::Rtc(rtc_select) => {
                        log::info!("Reading RTC {:?}", self.rtc_registers);
                        match rtc_select {
                            Seconds => self.rtc_registers.seconds,
                            Minutes => self.rtc_registers.minutes,
                            Hours => self.rtc_registers.hours,
                            Lower8bitsDayCounter => self.rtc_registers.lower_8bits_day_counter,
                            Upper1bitDayCounterCarryHalt => {
                                self.rtc_registers.upper_1bit_day_counter_carry_halt | 0b01111100
                            }
                        }
                    }
                }
            }
            _ => panic!(),
        }
    }
    fn write(&mut self, index: u16, value: u8) {
        match index {
            0x0000..=0x1fff => {
                if value == 0x0a {
                    self.ram_enabled = true
                } else if value == 0 {
                    self.ram_enabled = false;
                }
            }
            0x2000..=0x3fff => {
                let mut rom_bank = value as u16 & 0x7f;
                rom_bank &= u16::from(self.rom_bank_count) * 2 - 1;
                if rom_bank == 0 {
                    rom_bank = 1;
                }
                self.set_rom_bank(rom_bank);
            }
            0x4000..=0x5fff => {
                self.ram_rtc_select = match value {
                    0..0x08 => RamRtcSelect::Ram(value),
                    0x08 => RamRtcSelect::Rtc(RtcSelect::Seconds),
                    0x09 => RamRtcSelect::Rtc(RtcSelect::Minutes),
                    0x0a => RamRtcSelect::Rtc(RtcSelect::Hours),
                    0x0b => RamRtcSelect::Rtc(RtcSelect::Lower8bitsDayCounter),
                    0x0c => RamRtcSelect::Rtc(RtcSelect::Upper1bitDayCounterCarryHalt),
                    _ => self.ram_rtc_select,
                };
            }
            0x6000..VIDEO_RAM => {
                if self.latch_reg == 0 && value == 1 {
                    log::info!("Latch");
                    self.rtc_registers = self.rtc.get_clock_data()
                }
                self.latch_reg = value;
            }
            EXTERNAL_RAM..WORK_RAM => {
                if !self.ram_enabled {
                    return;
                }
                match self.ram_rtc_select {
                    RamRtcSelect::Ram(bank) => {
                        self.ram[usize::from(u16::from(bank) * RAM_BANK_SIZE)
                            + usize::from(index - EXTERNAL_RAM)] = value
                    }
                    RamRtcSelect::Rtc(rtc_select) => {
                        match rtc_select {
                            RtcSelect::Seconds => self.rtc_registers.seconds = value % 60,
                            RtcSelect::Minutes => self.rtc_registers.minutes = value % 60,
                            RtcSelect::Hours => self.rtc_registers.hours = value % 24,
                            RtcSelect::Lower8bitsDayCounter => {
                                self.rtc_registers.lower_8bits_day_counter = value
                            }
                            RtcSelect::Upper1bitDayCounterCarryHalt => {
                                self.rtc_registers.upper_1bit_day_counter_carry_halt = value
                            }
                        }
                        self.rtc.set_clock_data(self.rtc_registers);
                    }
                }
            }
            _ => panic!("Writing 0x{value:02x} to ${index:04x}"),
        }
    }

    fn load_saved_ram(&mut self, save: &[u8]) {
        let min = save.len().min(self.ram.len());
        self.ram[..min].copy_from_slice(&save[..min]);
    }

    // u16 -> days, u8 -> hours, u8 -> minutes
    fn load_additional_data(&mut self, additional_data: &[u8]) {
        self.rtc.deserialize(additional_data);
    }

    fn get_ram_to_save(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }

    fn get_additional_data_to_save(&self, buffer: &mut [u8]) -> usize {
        self.rtc.serialize(buffer)
    }

    fn get_rom(&self) -> &[u8] {
        &self.rom
    }
}
