pub const ROM_BANK: u16 = 0x0000;
pub const SWITCHABLE_ROM_BANK: u16 = 0x4000;
pub const VIDEO_RAM: u16 = 0x8000;
pub const EXTERNAL_RAM: u16 = 0xa000;
pub const WORK_RAM: u16 = 0xc000;
const ECHO_RAM: u16 = 0xe000;
pub const OAM: u16 = 0xfe00;
const NOT_USABLE: u16 = 0xfea0;
const JOYPAD: u16 = 0xff00;
const SB: u16 = 0xff01; // Serial transfer data
const SC: u16 = 0xff02; // Serial transfer control
const DIV: u16 = 0xff04; // Divider register (timer)
const TIMER_COUNTER: u16 = 0xff05; // TIMA
const TIMER_MODULO: u16 = 0xff06; // TMA
const TIMER_CONTROL: u16 = 0xff07; // TAC
const INTERRUPT_FLAG: u16 = 0xff0f;
const SWEEP: u16 = 0xff10;
const LENGTH_TIMER_AND_DUTY_CYCLE: u16 = 0xff11;
const VOLUME_AND_ENVELOPE: u16 = 0xff12;
const CHANNEL_1_PERIOD_LOW: u16 = 0xff13;
const CHANNEL_1_PERIOD_HIGH_AND_CONTROL: u16 = 0xff14;
const CHANNEL_3_DAC_ENABLE: u16 = 0xff1a;
const CHANNEL_3_OUTPUT_LEVEL: u16 = 0xff1c;
const CHANNEL_4_LENGTH_TIMER: u16 = 0xff20;
const CHANNEL_4_CONTROL: u16 = 0xff23;
const MASTER_VOLUME_AND_VIN_PANNING: u16 = 0xff24;
const SOUND_PANNING: u16 = 0xff25;
const AUDIO_MASTER_CONTROL: u16 = 0xff26;
const WAVE: u16 = 0xff30;
const LCD_CONTROL: u16 = 0xff40;
const LCD_STATUS: u16 = 0xff41;
const SCY: u16 = 0xff42;
const SCX: u16 = 0xff43;
const LY: u16 = 0xff44; // LCD Y
const LYC: u16 = 0xff45; // LY compare
const DMA: u16 = 0xff46;
const BGP: u16 = 0xff47;
const OBP0: u16 = 0xff48;
const OBP1: u16 = 0xff49;
const WY: u16 = 0xff4a;
const WX: u16 = 0xff4b;
const BOOT_ROM_MAPPING_CONTROL: u16 = 0xff50;
const HRAM: u16 = 0xff80;
const INTERRUPT_ENABLE: u16 = 0xffff;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    pub struct SerialControl: u8 {
        const TRANSFER_ENABLE = 1 << 7;
        const CLOCK_SELECT = 1;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    pub struct JoypadFlags: u8 {
        const NOT_BUTTONS = 1 << 5;
        const NOT_DPAD = 1 << 4;
        const NOT_START_DOWN = 1 << 3;
        const NOT_SELECT_UP = 1 << 2;
        const NOT_B_LEFT = 1 << 1;
        const NOT_A_RIGHT = 1;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    pub struct LcdStatus: u8 {
        const LYC_INT = 1 << 6;
        const OAM_INT = 1 << 5;
        const VBLANK_INT = 1 << 4;
        const HBLANK_INT = 1 << 3;
        const LYC_EQUAL_TO_LY = 1 << 2;
        // Drawing before ppu mask for debug output
        const DRAWING = 0b11;
        const PPU_MASK = 0b11;
        const HBLANK = 0;
        const VBLANK = 1;
        const OAM_SCAN = 0b10;
        const READONLY_MASK = 0b111;
    }
}

const DMG_BOOT: [u8; 256] = [
    49, 254, 255, 33, 255, 159, 175, 50, 203, 124, 32, 250, 14, 17, 33, 38, 255, 62, 128, 50, 226,
    12, 62, 243, 50, 226, 12, 62, 119, 50, 226, 17, 4, 1, 33, 16, 128, 26, 205, 184, 0, 26, 203,
    55, 205, 184, 0, 19, 123, 254, 52, 32, 240, 17, 204, 0, 6, 8, 26, 19, 34, 35, 5, 32, 249, 33,
    4, 153, 1, 12, 1, 205, 177, 0, 62, 25, 119, 33, 36, 153, 14, 12, 205, 177, 0, 62, 145, 224, 64,
    6, 16, 17, 212, 0, 120, 224, 67, 5, 123, 254, 216, 40, 4, 26, 224, 71, 19, 14, 28, 205, 167, 0,
    175, 144, 224, 67, 5, 14, 28, 205, 167, 0, 175, 176, 32, 224, 224, 67, 62, 131, 205, 159, 0,
    14, 39, 205, 167, 0, 62, 193, 205, 159, 0, 17, 138, 1, 240, 68, 254, 144, 32, 250, 27, 122,
    179, 32, 245, 24, 73, 14, 19, 226, 12, 62, 135, 226, 201, 240, 68, 254, 144, 32, 250, 13, 32,
    247, 201, 120, 34, 4, 13, 32, 250, 201, 71, 14, 4, 175, 197, 203, 16, 23, 193, 203, 16, 23, 13,
    32, 245, 34, 35, 34, 35, 201, 60, 66, 185, 165, 185, 165, 66, 60, 0, 84, 168, 252, 66, 79, 79,
    84, 73, 88, 46, 68, 77, 71, 32, 118, 49, 46, 50, 0, 62, 255, 198, 1, 11, 30, 216, 33, 77, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 1, 224, 80,
];

#[derive(Clone)]
pub struct State {
    pub boot_rom: &'static [u8; 256],
    pub mbc: Mbc,
    pub video_ram: [u8; (EXTERNAL_RAM - VIDEO_RAM) as usize],
    pub hram: [u8; (INTERRUPT_ENABLE - HRAM) as usize],
    pub wram: [u8; (ECHO_RAM - WORK_RAM) as usize],
    pub dma_register: u8,
    pub dma_request: bool,
    pub bgp_register: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub interrupt_enable: Ints,
    pub interrupt_flag: Ints,
    pub sweep: u8,
    pub length_timer_and_duty_cycle: u8,
    pub volume_and_envelope: u8,
    pub channel_1_period_low: u8,
    pub channel_1_period_high_and_control: u8,
    pub channel_3_dac_enable: u8,
    pub channel_3_output_level: u8,
    pub channel_4_length_timer: u8,
    pub channel_4_control: u8,
    pub master_volume_and_vin_panning: u8,
    pub sound_panning: u8,
    pub audio_master_control: u8,
    pub scy: u8,
    pub scx: u8,
    pub lcd_control: LcdControl,
    pub lcd_status: LcdStatus,
    pub ly: u8,
    pub lyc: u8,
    pub boot_rom_mapping_control: u8,
    pub sb: u8,
    pub sc: SerialControl,
    pub wy: u8,
    pub wx: u8,
    pub timer_modulo: u8,
    pub timer_control: u8,
    pub timer_counter: u8,
    pub oam: [u8; (NOT_USABLE - OAM) as usize],
    pub joypad: JoypadFlags,
    pub div: u8,
    pub reset_system_clock: bool,
}

impl State {
    pub fn mmu(&self) -> MmuRead<'_> {
        MmuRead(self)
    }
    pub fn new(rom: &'static [u8]) -> Self {
        Self {
            boot_rom: &DMG_BOOT,
            video_ram: [0; (EXTERNAL_RAM - VIDEO_RAM) as usize],
            hram: [0; (INTERRUPT_ENABLE - HRAM) as usize],
            wram: [0; (ECHO_RAM - WORK_RAM) as usize],
            dma_register: 0,
            dma_request: false,
            bgp_register: 0,
            obp0: 0,
            obp1: 0,
            interrupt_enable: Ints::empty(),
            interrupt_flag: Ints::empty(),
            sweep: 0,
            length_timer_and_duty_cycle: 0,
            volume_and_envelope: 0,
            channel_1_period_low: 0,
            channel_1_period_high_and_control: 0,
            channel_3_dac_enable: 0,
            channel_3_output_level: 0,
            channel_4_length_timer: 0,
            channel_4_control: 0,
            master_volume_and_vin_panning: 0,
            sound_panning: 0,
            audio_master_control: 0,
            scx: 0,
            scy: 0,
            lcd_control: LcdControl::empty(),
            ly: 0,
            lyc: 0,
            mbc: Mbc::new(rom),
            boot_rom_mapping_control: 0,
            sb: 0,
            sc: SerialControl::empty(),
            wy: 0,
            wx: 0,
            timer_modulo: 0,
            timer_control: 0,
            timer_counter: 0,
            lcd_status: LcdStatus::empty(),
            oam: [0; (NOT_USABLE - OAM) as usize],
            joypad: JoypadFlags::empty(),
            // https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register
            div: 0,
            reset_system_clock: false,
        }
    }
    pub fn set_interrupt_part_lcd_status(&mut self, value: u8) {
        self.lcd_status = (self.lcd_status & LcdStatus::READONLY_MASK)
            | (LcdStatus::from_bits_truncate(value) & !LcdStatus::READONLY_MASK)
    }
    pub fn get_data_for_write(&self) -> DataForWrite {
        DataForWrite {
            dma_request: self.dma_request,
            lcd_status: self.lcd_status,
        }
    }
}

use crate::{cartridge::Mbc, ic::Ints, ppu::LcdControl};

pub struct WriteOnlyState<'a>(&'a mut State);

impl<'a> WriteOnlyState<'a> {
    pub fn new(state: &'a mut State) -> Self {
        Self(state)
    }
    pub fn reborrow<'c>(&'c mut self) -> WriteOnlyState<'c>
    where
        'a: 'c,
    {
        WriteOnlyState(&mut *self.0)
    }
    pub fn mmu(&mut self, data_for_write: DataForWrite) -> MmuWrite<'_> {
        MmuWrite(self.0, data_for_write)
    }
    pub fn insert_ie(&mut self, flag: Ints) {
        self.0.interrupt_enable.insert(flag);
    }
    pub fn remove_ie(&mut self, flag: Ints) {
        self.0.interrupt_enable.remove(flag);
    }
    pub fn insert_if(&mut self, flag: Ints) {
        self.0.interrupt_flag.insert(flag);
    }
    pub fn remove_if(&mut self, flag: Ints) {
        self.0.interrupt_flag.remove(flag);
    }
    pub fn get_sc_mut(&mut self) -> &mut SerialControl {
        &mut self.0.sc
    }
    pub fn set_ly(&mut self, value: u8) {
        self.0.ly = value;
    }
    pub fn remove_if_bit(&mut self, bit: Ints) {
        self.0.interrupt_flag.remove(bit);
    }
    pub fn set_timer_counter(&mut self, timer_counter: u8) {
        self.0.timer_counter = timer_counter;
    }
    pub fn set_ppu_mode(&mut self, mode: LcdStatus) {
        self.0.lcd_status =
            (self.0.lcd_status & !LcdStatus::PPU_MASK) | (mode & LcdStatus::PPU_MASK);
    }
    pub fn set_interrupt_part_lcd_status(&mut self, value: u8) {
        self.0.set_interrupt_part_lcd_status(value);
    }

    pub fn write_to_oam(&mut self, index: u8, value: u8) {
        self.0.oam[usize::from(index)] = value;
    }

    pub fn set_dma_request_to_false(&mut self) {
        self.0.dma_request = false;
    }

    pub fn set_div(&mut self, value: u8) {
        self.0.div = value;
    }

    pub fn set_reset_system_clock(&mut self, value: bool) {
        self.0.reset_system_clock = value;
    }
}

pub struct MmuRead<'a>(&'a State);

pub struct MmuReadCpu<'a>(pub MmuRead<'a>);

impl MmuRead<'_> {
    pub fn read(&self, index: u16) -> u8 {
        match index {
            0..VIDEO_RAM => {
                if self.0.boot_rom_mapping_control == 0
                    && let Some(value) = self.0.boot_rom.get(usize::from(index)).copied()
                {
                    value
                } else {
                    self.0.mbc.read(index)
                }
            }
            VIDEO_RAM..EXTERNAL_RAM => {
                if (self.0.lcd_status & LcdStatus::PPU_MASK) == LcdStatus::DRAWING {
                    0xff
                } else {
                    self.0.video_ram[usize::from(index - VIDEO_RAM)]
                }
            }
            EXTERNAL_RAM..WORK_RAM => self.0.mbc.read(index),
            WORK_RAM..ECHO_RAM => self.0.wram[usize::from(index - WORK_RAM)],
            ECHO_RAM..OAM => self.0.wram[usize::from(index - ECHO_RAM)],
            OAM..NOT_USABLE => {
                let ppu = self.0.lcd_status & LcdStatus::PPU_MASK;
                if ppu == LcdStatus::DRAWING || ppu == LcdStatus::OAM_SCAN {
                    0xff
                } else {
                    self.0.oam[usize::from(index - OAM)]
                }
            }
            JOYPAD => {
                (if self
                    .0
                    .joypad
                    .contains(JoypadFlags::NOT_BUTTONS | JoypadFlags::NOT_DPAD)
                {
                    // https://gbdev.io/pandocs/Joypad_Input.html#ff00--p1joyp-joypad
                    self.0.joypad.bits() | 0xf
                } else {
                    self.0.joypad.bits()
                }) | 0b11000000 // unused bits return 1
            }
            SB => self.0.sb,
            SC => self.0.sc.bits() | 0b01111110,
            0xff03 => 0xff,
            DIV => self.0.div,
            TIMER_COUNTER => self.0.timer_counter,
            TIMER_MODULO => self.0.timer_modulo,
            TIMER_CONTROL => self.0.timer_control | 0b11111000,
            0xff08..INTERRUPT_FLAG => 0xff,
            INTERRUPT_FLAG => self.0.interrupt_flag.bits() | 0b11100000,
            SWEEP => self.0.sweep | 0b10000000,
            LENGTH_TIMER_AND_DUTY_CYCLE => self.0.length_timer_and_duty_cycle,
            VOLUME_AND_ENVELOPE => self.0.volume_and_envelope,
            CHANNEL_1_PERIOD_LOW => 0xff,
            CHANNEL_1_PERIOD_HIGH_AND_CONTROL => {
                self.0.channel_1_period_high_and_control | 0b10111111
            }
            0xff15 => 0xff,
            CHANNEL_3_DAC_ENABLE => self.0.channel_3_dac_enable | 0b01111111,
            CHANNEL_3_OUTPUT_LEVEL => self.0.channel_3_output_level | 0b10011111,
            0xff1f => 0xff,
            CHANNEL_4_LENGTH_TIMER => 0xff,
            CHANNEL_4_CONTROL => self.0.channel_4_control | 0b10111111,
            MASTER_VOLUME_AND_VIN_PANNING => self.0.master_volume_and_vin_panning,
            SOUND_PANNING => self.0.sound_panning,
            AUDIO_MASTER_CONTROL => self.0.audio_master_control | 0b01110000,
            0xff27..WAVE => 0xff,
            LCD_CONTROL => self.0.lcd_control.bits(),
            LCD_STATUS => {
                let mut status = self.0.lcd_status;
                // https://gbdev.io/pandocs/STAT.html#ff41--stat-lcd-status
                status.set(LcdStatus::LYC_EQUAL_TO_LY, self.0.ly == self.0.lyc);
                status.bits() | 0b10000000
            }
            SCY => self.0.scy,
            SCX => self.0.scx,
            LY => self.0.ly,
            LYC => self.0.lyc,
            DMA => self.0.dma_register,
            BGP => self.0.bgp_register,
            OBP0 => self.0.obp0,
            OBP1 => self.0.obp1,
            WY => self.0.wy,
            WX => self.0.wx,
            0xff4c => 0xff,
            0xff4d => {
                log::warn!("Reading $ff4d (Prepare speed switch)");
                0xff
            }
            0xff4e => 0xff,
            0xff4f => 0xff,
            BOOT_ROM_MAPPING_CONTROL => 0xff,
            0xff51..HRAM => 0xff,
            HRAM..INTERRUPT_ENABLE => self.0.hram[usize::from(index - HRAM)],
            INTERRUPT_ENABLE => self.0.interrupt_enable.bits(),
            _ => todo!("Reading ${index:04x}"),
        }
    }
}

impl<'a> MmuReadCpu<'a> {
    pub fn read(&self, index: u16) -> u8 {
        if self.0.0.dma_request && (OAM..NOT_USABLE).contains(&index) {
            return 0xff;
        }
        self.0.read(index)
    }
}

pub struct DataForWrite {
    dma_request: bool,
    lcd_status: LcdStatus,
}

// never read State for taking decisions (like conditions) when writing. Use DataForWrite instead.
pub struct MmuWrite<'a>(&'a mut State, DataForWrite);

impl MmuWrite<'_> {
    pub fn inner(&mut self) -> WriteOnlyState<'_> {
        WriteOnlyState(self.0)
    }
    pub fn write(&mut self, index: u16, value: u8) {
        if self.1.dma_request && !(HRAM..INTERRUPT_ENABLE).contains(&index) {
            return;
        }

        match index {
            0..VIDEO_RAM => self.0.mbc.write(index, value),
            VIDEO_RAM..EXTERNAL_RAM => {
                if (self.1.lcd_status & LcdStatus::PPU_MASK) != LcdStatus::DRAWING {
                    self.0.video_ram[usize::from(index - VIDEO_RAM)] = value
                }
            }
            EXTERNAL_RAM..WORK_RAM => self.0.mbc.write(index, value),
            WORK_RAM..ECHO_RAM => self.0.wram[usize::from(index - WORK_RAM)] = value,
            ECHO_RAM..OAM => self.0.wram[usize::from(index - ECHO_RAM)] = value,
            OAM..NOT_USABLE => {
                let ppu = self.1.lcd_status & LcdStatus::PPU_MASK;
                if ppu != LcdStatus::DRAWING && ppu != LcdStatus::OAM_SCAN {
                    self.0.oam[usize::from(index - OAM)] = value
                }
            }
            JOYPAD => {
                self.0
                    .joypad
                    .remove(JoypadFlags::NOT_BUTTONS | JoypadFlags::NOT_DPAD);
                self.0.joypad |= JoypadFlags::from_bits_retain(value)
                    & (JoypadFlags::NOT_BUTTONS | JoypadFlags::NOT_DPAD)
            }
            SB => self.0.sb = value,
            SC => self.0.sc = SerialControl::from_bits_truncate(value),
            0xff03 => {}
            // Citation:
            // Writing any value to this register resets it to $00
            DIV => self.0.div = 0,
            TIMER_COUNTER => self.0.timer_counter = value,
            TIMER_MODULO => self.0.timer_modulo = value,
            TIMER_CONTROL => self.0.timer_control = value,
            0xff08..INTERRUPT_FLAG => {}
            INTERRUPT_FLAG => self.0.interrupt_flag = Ints::from_bits_truncate(value),
            SWEEP => self.0.sweep = value,
            LENGTH_TIMER_AND_DUTY_CYCLE => self.0.length_timer_and_duty_cycle = value,
            VOLUME_AND_ENVELOPE => self.0.volume_and_envelope = value,
            CHANNEL_1_PERIOD_LOW => self.0.channel_1_period_low = value,
            CHANNEL_1_PERIOD_HIGH_AND_CONTROL => self.0.channel_1_period_high_and_control = value,
            0xff15 => {}
            CHANNEL_3_DAC_ENABLE => self.0.channel_3_dac_enable = value,
            CHANNEL_3_OUTPUT_LEVEL => self.0.channel_3_output_level = value,
            0xff1f => {}
            CHANNEL_4_LENGTH_TIMER => self.0.channel_4_length_timer = value,
            CHANNEL_4_CONTROL => self.0.channel_4_control = value,
            MASTER_VOLUME_AND_VIN_PANNING => self.0.master_volume_and_vin_panning = value,
            SOUND_PANNING => self.0.sound_panning = value,
            AUDIO_MASTER_CONTROL => self.0.audio_master_control = value,
            0xff27..WAVE => {}
            WAVE..LCD_CONTROL => {
                // TODO wave ram
            }
            LCD_CONTROL => self.0.lcd_control = LcdControl::from_bits_truncate(value),
            // https://gbdev.io/pandocs/STAT.html#ff41--stat-lcd-status 3 last bits readonly
            LCD_STATUS => self.0.set_interrupt_part_lcd_status(value),
            SCY => {
                // println!("SCY {value:x}");
                self.0.scy = value
            }
            SCX => self.0.scx = value,
            LY => {} // read only
            LYC => self.0.lyc = value,
            DMA => {
                self.0.dma_register = value;
                self.0.dma_request = true;
            }
            BGP => self.0.bgp_register = value,
            OBP0 => self.0.obp0 = value,
            OBP1 => self.0.obp1 = value,
            WY => self.0.wy = value,
            WX => self.0.wx = value,
            0xff4c => {}
            0xff4d => {
                log::warn!("Writing $ff4d (Prepare speed switch)");
            }
            0xff4e => {}
            0xff4f => {}
            BOOT_ROM_MAPPING_CONTROL => self.0.boot_rom_mapping_control = value,
            0xff51..HRAM => {}
            HRAM..INTERRUPT_ENABLE => self.0.hram[usize::from(index - HRAM)] = value,
            INTERRUPT_ENABLE => self.0.interrupt_enable = Ints::from_bits_retain(value),
            _ => todo!("Writing 0x{value:02x} at ${index:04x}"),
        }
    }
}
