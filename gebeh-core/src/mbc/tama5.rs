// originally from https://github.com/mgba-emu/mgba/blob/79fa503d63a2ebb56487d02a9e0d74d455d0149a/src/gb/mbc/tama5.c
// original license: https://www.mozilla.org/en-US/MPL/2.0/
// used GLM 5 to first convert the C code into Rust

use crate::{mbc::*, state::*};
use core::ops::Deref;

const GBTAMA5_BANK_LO: u8 = 0x0;
const GBTAMA5_BANK_HI: u8 = 0x1;
const GBTAMA5_WRITE_LO: u8 = 0x4;
const GBTAMA5_WRITE_HI: u8 = 0x5;
const GBTAMA5_ADDR_HI: u8 = 0x6;
const GBTAMA5_ADDR_LO: u8 = 0x7;
const GBTAMA5_MAX: u8 = 0x8;
const GBTAMA5_ACTIVE: u8 = 0xA;
const GBTAMA5_READ_LO: u8 = 0xC;
const GBTAMA5_READ_HI: u8 = 0xD;

const GBTAMA6_RTC_PA0_SECOND_1: u8 = 0x0;
const GBTAMA6_RTC_PA0_SECOND_10: u8 = 0x1;
const GBTAMA6_RTC_PA0_MINUTE_1: u8 = 0x2;
const GBTAMA6_RTC_PA0_MINUTE_10: u8 = 0x3;
const GBTAMA6_RTC_PA0_HOUR_1: u8 = 0x4;
const GBTAMA6_RTC_PA0_HOUR_10: u8 = 0x5;
// const GBTAMA6_RTC_PA0_WEEK: u8 = 0x6;
// const GBTAMA6_RTC_PA0_DAY_1: u8 = 0x7;
// const GBTAMA6_RTC_PA0_DAY_10: u8 = 0x8;
// const GBTAMA6_RTC_PA0_MONTH_1: u8 = 0x9;
// const GBTAMA6_RTC_PA0_MONTH_10: u8 = 0xA;
// const GBTAMA6_RTC_PA0_YEAR_1: u8 = 0xB;
// const GBTAMA6_RTC_PA0_YEAR_10: u8 = 0xC;
// const GBTAMA6_RTC_PA1_MINUTE_1: u8 = 0x2;
// const GBTAMA6_RTC_PA1_MINUTE_10: u8 = 0x3;
// const GBTAMA6_RTC_PA1_HOUR_1: u8 = 0x4;
// const GBTAMA6_RTC_PA1_HOUR_10: u8 = 0x5;
// const GBTAMA6_RTC_PA1_WEEK: u8 = 0x6;
// const GBTAMA6_RTC_PA1_DAY_1: u8 = 0x7;
// const GBTAMA6_RTC_PA1_DAY_10: u8 = 0x8;
// const GBTAMA6_RTC_PA1_24_HOUR: u8 = 0xA;
// const GBTAMA6_RTC_PA1_LEAP_YEAR: u8 = 0xB;
const GBTAMA6_RTC_PAGE: u8 = 0xD;
// const GBTAMA6_RTC_TEST: u8 = 0xE;
// const GBTAMA6_RTC_RESET: u8 = 0xF;
const GBTAMA6_RTC_MAX: u8 = 0x10;

const GBTAMA6_DISABLE_TIMER: u8 = 0x0;
const GBTAMA6_ENABLE_TIMER: u8 = 0x1;
const GBTAMA6_MINUTE_WRITE: u8 = 0x4;
const GBTAMA6_HOUR_WRITE: u8 = 0x5;
const GBTAMA6_MINUTE_READ: u8 = 0x6;
const GBTAMA6_HOUR_READ: u8 = 0x7;
const GBTAMA6_DISABLE_ALARM: u8 = 0x10;
const GBTAMA6_ENABLE_ALARM: u8 = 0x11;

static TAMA6_RTC_MASK: [u8; 32] = [
    0xF, 0x7, 0xF, 0x7, 0xF, 0x3, 0x7, 0xF, 0x3, 0xF, 0x1, 0xF, 0xF, 0x0, 0x0, 0x0, 0x0, 0x0, 0xF,
    0x7, 0xF, 0x3, 0x7, 0xF, 0x3, 0x0, 0x1, 0x3, 0x0, 0x0, 0x0, 0x0,
];

#[derive(Clone)]
pub struct Tama5State {
    pub registers: [u8; GBTAMA5_MAX as usize],
    pub reg: u8,
    pub rom_bank: u8,
    pub rtc_timer_page: [u8; GBTAMA6_RTC_MAX as usize],
    pub rtc_alarm_page: [u8; GBTAMA6_RTC_MAX as usize],
    pub rtc_free_page0: [u8; GBTAMA6_RTC_MAX as usize],
    pub rtc_free_page1: [u8; GBTAMA6_RTC_MAX as usize],
    pub disabled: bool,
    pub rtc_last_latch: i64,
}

impl Default for Tama5State {
    fn default() -> Self {
        let mut rtc_alarm_page: [u8; _] = Default::default();
        let mut rtc_free_page0: [u8; _] = Default::default();
        let mut rtc_free_page1: [u8; _] = Default::default();
        rtc_alarm_page[usize::from(GBTAMA6_RTC_PAGE)] = 1;
        rtc_free_page0[usize::from(GBTAMA6_RTC_PAGE)] = 2;
        rtc_free_page1[usize::from(GBTAMA6_RTC_PAGE)] = 3;

        Self {
            registers: Default::default(),
            reg: Default::default(),
            rom_bank: Default::default(),
            rtc_timer_page: Default::default(),
            rtc_alarm_page,
            rtc_free_page0,
            rtc_free_page1,
            disabled: Default::default(),
            rtc_last_latch: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct Tama5<T> {
    rom: T,
    ram: [u8; 0x20],
    state: Tama5State,
}

impl<T: Deref<Target = [u8]>> Tama5<T> {
    pub fn new(rom: T) -> Self {
        Self {
            rom,
            ram: [0; 0x20],
            state: Tama5State::default(),
        }
    }

    fn get_rtc_address(&self) -> u8 {
        ((self.state.registers[usize::from(GBTAMA5_ADDR_HI)] << 4) & 0x10)
            | self.state.registers[usize::from(GBTAMA5_ADDR_LO)]
    }

    fn get_write_value(&self) -> u8 {
        (self.state.registers[usize::from(GBTAMA5_WRITE_HI)] << 4)
            | self.state.registers[usize::from(GBTAMA5_WRITE_LO)]
    }

    fn handle_write(&mut self, value: u8) {
        let value = value & 0xF;
        if self.state.reg >= GBTAMA5_MAX {
            return;
        }

        self.state.registers[usize::from(self.state.reg)] = value;
        let address = self.get_rtc_address();
        let out = self.get_write_value();

        match self.state.reg {
            GBTAMA5_BANK_LO | GBTAMA5_BANK_HI => {
                self.state.rom_bank = self.state.registers[usize::from(GBTAMA5_BANK_LO)]
                    | (self.state.registers[usize::from(GBTAMA5_BANK_HI)] << 4);
            }
            GBTAMA5_WRITE_LO | GBTAMA5_WRITE_HI | GBTAMA5_ADDR_HI => {}
            GBTAMA5_ADDR_LO => {
                match self.state.registers[usize::from(GBTAMA5_ADDR_HI)] >> 1 {
                    0x0 => {
                        // RAM write
                        self.ram[usize::from(address)] = out;
                    }
                    0x1 => {
                        // RAM read - nothing to do
                    }
                    0x2 => {
                        // Other commands
                        match address {
                            GBTAMA6_DISABLE_TIMER => {
                                self.state.disabled = true;
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PAGE)] &= 0x7;
                                self.state.rtc_alarm_page[usize::from(GBTAMA6_RTC_PAGE)] &= 0x7;
                                self.state.rtc_free_page0[usize::from(GBTAMA6_RTC_PAGE)] &= 0x7;
                                self.state.rtc_free_page1[usize::from(GBTAMA6_RTC_PAGE)] &= 0x7;
                            }
                            GBTAMA6_ENABLE_TIMER => {
                                self.state.disabled = false;
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PA0_SECOND_1)] =
                                    0;
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PA0_SECOND_10)] =
                                    0;
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PAGE)] |= 0x8;
                                self.state.rtc_alarm_page[usize::from(GBTAMA6_RTC_PAGE)] |= 0x8;
                                self.state.rtc_free_page0[usize::from(GBTAMA6_RTC_PAGE)] |= 0x8;
                                self.state.rtc_free_page1[usize::from(GBTAMA6_RTC_PAGE)] |= 0x8;
                            }
                            GBTAMA6_MINUTE_WRITE => {
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PA0_MINUTE_1)] =
                                    out & 0xF;
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PA0_MINUTE_10)] =
                                    out >> 4;
                            }
                            GBTAMA6_HOUR_WRITE => {
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PA0_HOUR_1)] =
                                    out & 0xF;
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PA0_HOUR_10)] =
                                    out >> 4;
                            }
                            GBTAMA6_DISABLE_ALARM => {
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PAGE)] &= 0xB;
                                self.state.rtc_alarm_page[usize::from(GBTAMA6_RTC_PAGE)] &= 0xB;
                                self.state.rtc_free_page0[usize::from(GBTAMA6_RTC_PAGE)] &= 0xB;
                                self.state.rtc_free_page1[usize::from(GBTAMA6_RTC_PAGE)] &= 0xB;
                            }
                            GBTAMA6_ENABLE_ALARM => {
                                self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PAGE)] |= 0x4;
                                self.state.rtc_alarm_page[usize::from(GBTAMA6_RTC_PAGE)] |= 0x4;
                                self.state.rtc_free_page0[usize::from(GBTAMA6_RTC_PAGE)] |= 0x4;
                                self.state.rtc_free_page1[usize::from(GBTAMA6_RTC_PAGE)] |= 0x4;
                            }
                            _ => {}
                        }
                    }
                    0x4 => {
                        // RTC access
                        let rtc_addr = self.state.registers[usize::from(GBTAMA5_WRITE_LO)];
                        if rtc_addr >= GBTAMA6_RTC_PAGE {
                            return;
                        }
                        let out = self.state.registers[usize::from(GBTAMA5_WRITE_HI)];
                        match self.state.registers[usize::from(GBTAMA5_ADDR_LO)] {
                            0 => {
                                let masked = out & TAMA6_RTC_MASK[usize::from(rtc_addr)];
                                self.state.rtc_timer_page[usize::from(rtc_addr)] = masked;
                            }
                            2 => {
                                let masked = out & TAMA6_RTC_MASK[usize::from(rtc_addr | 0x10)];
                                self.state.rtc_alarm_page[usize::from(rtc_addr)] = masked;
                            }
                            4 => {
                                self.state.rtc_free_page0[usize::from(rtc_addr)] = out;
                            }
                            6 => {
                                self.state.rtc_free_page1[usize::from(rtc_addr)] = out;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl<T: Deref<Target = [u8]>> Mbc for Tama5<T> {
    fn read(&self, index: u16) -> u8 {
        match index {
            ROM_BANK..SWITCHABLE_ROM_BANK => self.rom[usize::from(index)],
            SWITCHABLE_ROM_BANK..VIDEO_RAM => {
                let bank = if self.state.rom_bank == 0 {
                    1
                } else {
                    self.state.rom_bank
                };
                let offset = usize::from(bank) * usize::from(ROM_BANK_SIZE);
                self.rom
                    .get(offset + (index - 0x4000) as usize)
                    .copied()
                    .unwrap_or(0)
            }
            0xA000 => match self.state.reg {
                GBTAMA5_ACTIVE => 0xF1,
                GBTAMA5_READ_LO | GBTAMA5_READ_HI => {
                    let address = self.get_rtc_address();
                    let mut value: u8 = 0xF0;

                    match self.state.registers[usize::from(GBTAMA5_ADDR_HI)] >> 1 {
                        0x1 => {
                            value = self.ram[usize::from(address)];
                        }
                        0x2 => {
                            // RTC read - would need to latch here
                            match address {
                                GBTAMA6_MINUTE_READ => {
                                    value = (self.state.rtc_timer_page
                                        [usize::from(GBTAMA6_RTC_PA0_MINUTE_10)]
                                        << 4)
                                        | self.state.rtc_timer_page
                                            [usize::from(GBTAMA6_RTC_PA0_MINUTE_1)];
                                }
                                GBTAMA6_HOUR_READ => {
                                    value = (self.state.rtc_timer_page
                                        [usize::from(GBTAMA6_RTC_PA0_HOUR_10)]
                                        << 4)
                                        | self.state.rtc_timer_page
                                            [usize::from(GBTAMA6_RTC_PA0_HOUR_1)];
                                }
                                _ => {
                                    value = address;
                                }
                            }
                        }
                        0x4 => {
                            if self.state.reg == GBTAMA5_READ_HI {
                                return 0xF1;
                            }
                            let rtc_addr = self.state.registers[usize::from(GBTAMA5_WRITE_LO)];
                            if rtc_addr > GBTAMA6_RTC_PAGE {
                                return 0xF0;
                            }
                            if core::matches!(
                                self.state.registers[usize::from(GBTAMA5_ADDR_LO)],
                                1 | 3 | 5 | 7
                            ) {
                                value = self.state.rtc_timer_page[usize::from(rtc_addr)];
                            }
                        }
                        _ => {}
                    }

                    if self.state.reg == GBTAMA5_READ_HI {
                        value >>= 4;
                    }
                    value | 0xF0
                }
                _ => 0xF1,
            },
            0xA001 => 0xff,
            0xA002..WORK_RAM => self.ram[usize::from(index) - usize::from(EXTERNAL_RAM)],
            _ => panic!(),
        }
    }

    fn write(&mut self, index: u16, value: u8) {
        match index {
            0xA000..0xA002 => {
                if index & 1 != 0 {
                    // A001 - Register select
                    self.state.reg = value;
                } else {
                    // A000 - Register write
                    self.handle_write(value);
                }
            }
            0xA002..WORK_RAM => {
                self.ram[usize::from(index) - usize::from(EXTERNAL_RAM)] = value;
            }
            _ => {}
        }
    }

    fn load_saved_ram(&mut self, save: &[u8]) {
        let min = save.len().min(self.ram.len());
        self.ram[..min].copy_from_slice(&save[..min]);
    }

    fn load_additional_data(&mut self, additional_data: &[u8]) {
        // Expected format: 32 bytes (4 pages * 8 bytes each) + 8 bytes for latched time
        if additional_data.len() < 40 {
            return;
        }

        for i in 0..8 {
            self.state.rtc_timer_page[i * 2] = additional_data[i] & 0xF;
            self.state.rtc_timer_page[i * 2 + 1] = additional_data[i] >> 4;
            self.state.rtc_alarm_page[i * 2] = additional_data[i + 8] & 0xF;
            self.state.rtc_alarm_page[i * 2 + 1] = additional_data[i + 8] >> 4;
            self.state.rtc_free_page0[i * 2] = additional_data[i + 16] & 0xF;
            self.state.rtc_free_page0[i * 2 + 1] = additional_data[i + 16] >> 4;
            self.state.rtc_free_page1[i * 2] = additional_data[i + 24] & 0xF;
            self.state.rtc_free_page1[i * 2 + 1] = additional_data[i + 24] >> 4;
        }

        // Load latched Unix time (little-endian 64-bit)
        self.state.rtc_last_latch =
            i64::from_le_bytes(additional_data[32..40].try_into().unwrap_or([0; 8]));

        self.state.disabled = (self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PAGE)] & 0x8) == 0;

        self.state.rtc_timer_page[usize::from(GBTAMA6_RTC_PAGE)] &= 0xC;
        self.state.rtc_alarm_page[usize::from(GBTAMA6_RTC_PAGE)] &= 0xC;
        self.state.rtc_alarm_page[usize::from(GBTAMA6_RTC_PAGE)] |= 1;
        self.state.rtc_free_page0[usize::from(GBTAMA6_RTC_PAGE)] &= 0xC;
        self.state.rtc_free_page0[usize::from(GBTAMA6_RTC_PAGE)] |= 2;
        self.state.rtc_free_page1[usize::from(GBTAMA6_RTC_PAGE)] &= 0xC;
        self.state.rtc_free_page1[usize::from(GBTAMA6_RTC_PAGE)] |= 3;
    }

    fn get_ram_to_save(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }

    fn get_additional_data_to_save(&self, buffer: &mut [u8]) -> usize {
        if buffer.len() < 40 {
            panic!("Buffer too small for TAMA5 save data");
        }

        for i in 0..8 {
            buffer[i] = self.state.rtc_timer_page[i * 2] & 0xF
                | (self.state.rtc_timer_page[i * 2 + 1] << 4);
            buffer[i + 8] = self.state.rtc_alarm_page[i * 2] & 0xF
                | (self.state.rtc_alarm_page[i * 2 + 1] << 4);
            buffer[i + 16] = self.state.rtc_free_page0[i * 2] & 0xF
                | (self.state.rtc_free_page0[i * 2 + 1] << 4);
            buffer[i + 24] = self.state.rtc_free_page1[i * 2] & 0xF
                | (self.state.rtc_free_page1[i * 2 + 1] << 4);
        }

        buffer[32..40].copy_from_slice(&self.state.rtc_last_latch.to_le_bytes());

        40
    }

    fn get_rom(&self) -> &[u8] {
        &self.rom
    }
}
