use crate::{mbc::Mbc, ppu::LcdControl};

pub const ROM_BANK: u16 = 0x0000;
pub const SWITCHABLE_ROM_BANK: u16 = 0x4000;
pub const VIDEO_RAM: u16 = 0x8000;
pub const EXTERNAL_RAM: u16 = 0xa000;
pub const WORK_RAM: u16 = 0xc000;
pub const ECHO_RAM: u16 = 0xe000;
pub const OAM: u16 = 0xfe00;
pub const NOT_USABLE: u16 = 0xfea0;
pub const JOYPAD: u16 = 0xff00;
pub const SB: u16 = 0xff01; // Serial transfer data
pub const SC: u16 = 0xff02; // Serial transfer control
pub const DIV: u16 = 0xff04; // Divider register (timer)
pub const TIMER_COUNTER: u16 = 0xff05; // TIMA
pub const TIMER_MODULO: u16 = 0xff06; // TMA
pub const TIMER_CONTROL: u16 = 0xff07; // TAC
pub const INTERRUPT_FLAG: u16 = 0xff0f;
pub const CH1_SWEEP: u16 = 0xff10;
pub const CH1_LENGTH_TIMER_AND_DUTY_CYCLE: u16 = 0xff11;
pub const CH1_VOLUME_AND_ENVELOPE: u16 = 0xff12;
pub const CH1_PERIOD_LOW: u16 = 0xff13;
pub const CH1_PERIOD_HIGH_AND_CONTROL: u16 = 0xff14;
pub const CH2_LENGTH_TIMER_AND_DUTY_CYCLE: u16 = 0xff16;
pub const CH2_VOLUME_AND_ENVELOPE: u16 = 0xff17;
pub const CH2_PERIOD_LOW: u16 = 0xff18;
pub const CH2_PERIOD_HIGH_AND_CONTROL: u16 = 0xff19;
pub const CH3_DAC_ENABLE: u16 = 0xff1a;
pub const CH3_LENGTH_TIMER: u16 = 0xff1b;
pub const CH3_OUTPUT_LEVEL: u16 = 0xff1c;
pub const CH3_PERIOD_HIGH_AND_CONTROL: u16 = 0xff1e;
pub const CH3_PERIOD_LOW: u16 = 0xff1d;
pub const CH4_LENGTH_TIMER: u16 = 0xff20;
pub const CH4_VOLUME_AND_ENVELOPE: u16 = 0xff21;
pub const CH4_FREQUENCY_AND_RANDOMNESS: u16 = 0xff22;
pub const CH4_CONTROL: u16 = 0xff23;
pub const MASTER_VOLUME_AND_VIN_PANNING: u16 = 0xff24;
pub const SOUND_PANNING: u16 = 0xff25;
pub const AUDIO_MASTER_CONTROL: u16 = 0xff26;
pub const WAVE: u16 = 0xff30;
pub const LCD_CONTROL: u16 = 0xff40;
pub const LCD_STATUS: u16 = 0xff41;
pub const SCY: u16 = 0xff42;
pub const SCX: u16 = 0xff43;
pub const LY: u16 = 0xff44; // LCD Y
pub const LYC: u16 = 0xff45; // LY compare
pub const DMA: u16 = 0xff46;
pub const BGP: u16 = 0xff47;
pub const OBP0: u16 = 0xff48;
pub const OBP1: u16 = 0xff49;
pub const WY: u16 = 0xff4a;
pub const WX: u16 = 0xff4b;
pub const BOOT_ROM_MAPPING_CONTROL: u16 = 0xff50;
pub const HRAM: u16 = 0xff80;
pub const INTERRUPT_ENABLE: u16 = 0xffff;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    pub struct SerialControl: u8 {
        const TRANSFER_ENABLE = 1 << 7;
        const CLOCK_SELECT = 1;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq, Default)]
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

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct Interruptions: u8 {
        const VBLANK = 1;
        const LCD = 1 << 1;
        const TIMER = 1 << 2;
        const SERIAL = 1 << 3;
        const JOYPAD = 1 << 4;
    }
}

#[cfg(test)]
mod tests {
    use super::Interruptions;

    #[test]
    fn good_priority() {
        let ints = Interruptions::all();
        let mut ints = ints.iter();
        assert_eq!(Some(Interruptions::VBLANK), ints.next());
        assert_eq!(Some(Interruptions::LCD), ints.next());
        assert_eq!(Some(Interruptions::TIMER), ints.next());
        assert_eq!(Some(Interruptions::SERIAL), ints.next());
        assert_eq!(Some(Interruptions::JOYPAD), ints.next());
        assert_eq!(None, ints.next());
    }
}

// from https://github.com/Ashiepaws/Bootix
pub const BOOTIX_BOOT_ROM: [u8; 256] = [
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

// if read by the cpu the same cycle they are written, then the cpu will read the old value.
// The delayed value will be read the next cycle.
// Be careful, delayed state can be written over the same cycle by the CPU thus it will never be read.
#[derive(Clone, Default)]
pub struct Delayed {
    // according to some mooneye tests, interrupts from PPU are delayed by one M-cycle
    pub interrupt_flag: Interruptions,
    // according to some mooneye tests and a comment in SameBoy PPU implementation, STAT mode is delayed by one M-cycle
    // https://github.com/LIJI32/SameBoy/blob/858f0034650fc91778f2cf9adaf801ce77d2fe68/Core/display.c#L1530
    pub ppu_mode: LcdStatus,
}

#[derive(Clone)]
pub struct State {
    pub video_ram: [u8; (EXTERNAL_RAM - VIDEO_RAM) as usize],
    pub wram: [u8; (ECHO_RAM - WORK_RAM) as usize],
    pub dma_register: u8,
    pub dma_request: bool,
    pub is_dma_active: bool,
    pub bgp_register: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub interrupt_flag: Interruptions,
    pub sound_panning: u8,
    pub audio_master_control: u8,
    pub scy: u8,
    pub scx: u8,
    pub lcd_control: LcdControl,
    pub lcd_status: LcdStatus,
    pub ly: u8,
    pub lyc: u8,
    pub sb: u8,
    pub sc: SerialControl,
    pub wy: u8,
    pub wx: u8,
    pub oam: [u8; (NOT_USABLE - OAM) as usize],
    pub delayed: Delayed,
    // https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
}

#[derive(Clone, Copy, Default)]
pub struct Scrolling {
    // 0 < x < 256
    pub x: u8,
    // 0 < y < 256
    pub y: u8,
}

impl Default for State {
    fn default() -> Self {
        Self {
            video_ram: [0; 0x2000],
            wram: [0; (ECHO_RAM - WORK_RAM) as usize],
            dma_register: 0,
            dma_request: false,
            is_dma_active: false,
            bgp_register: 0,
            obp0: 0,
            obp1: 0,
            interrupt_flag: Interruptions::empty(),
            sound_panning: 0,
            audio_master_control: 0,
            scx: 0,
            scy: 0,
            lcd_control: LcdControl::empty(),
            ly: 0,
            lyc: 0,
            sb: 0,
            sc: SerialControl::empty(),
            wy: 0,
            wx: 0,
            lcd_status: LcdStatus::empty(),
            oam: [0; (NOT_USABLE - OAM) as usize],
            // https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register
            delayed: Default::default(),
        }
    }
}

impl State {
    pub fn set_interrupt_part_lcd_status(&mut self, value: u8) {
        self.lcd_status = (self.lcd_status & LcdStatus::READONLY_MASK)
            | (LcdStatus::from_bits_truncate(value) & !LcdStatus::READONLY_MASK)
    }
    fn set_ppu_mode(&mut self, mode: LcdStatus) {
        assert!(matches!(
            mode,
            LcdStatus::VBLANK | LcdStatus::HBLANK | LcdStatus::DRAWING | LcdStatus::OAM_SCAN
        ));
        self.lcd_status = (self.lcd_status & !LcdStatus::PPU_MASK) | (mode & LcdStatus::PPU_MASK);
    }
    pub fn apply_delayed(&mut self) {
        self.set_ppu_mode(self.delayed.ppu_mode);
        self.interrupt_flag |= self.delayed.interrupt_flag;
        self.delayed.interrupt_flag = Interruptions::empty();
    }
    pub fn get_scrolling(&self) -> Scrolling {
        Scrolling {
            x: self.scx,
            y: self.scy,
        }
    }
}

pub trait MmuExt {
    fn read<M: Mbc + ?Sized>(&self, index: u16, mbc: &M) -> u8;
}

impl MmuExt for State {
    fn read<M: Mbc + ?Sized>(&self, index: u16, mbc: &M) -> u8 {
        match index {
            0..VIDEO_RAM => mbc.read(index),
            VIDEO_RAM..EXTERNAL_RAM => {
                if (self.lcd_status & LcdStatus::PPU_MASK) == LcdStatus::DRAWING {
                    0xff
                } else {
                    self.video_ram[usize::from(index - VIDEO_RAM)]
                }
            }
            EXTERNAL_RAM..WORK_RAM => mbc.read(index),
            WORK_RAM..ECHO_RAM => self.wram[usize::from(index - WORK_RAM)],
            // if greater than 0xdfff then the dma has access to a bigger echo ram than the cpu
            // from https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/hardware.rs#L215
            ECHO_RAM.. => self.wram[usize::from(index - ECHO_RAM)],
        }
    }
}
