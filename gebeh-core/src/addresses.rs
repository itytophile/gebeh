use crate::{Wram, mbc::Mbc, ppu::Vram};

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

pub fn mmu_read<M: Mbc + ?Sized>(index: u16, mbc: &M, vram: &Vram, wram: &Wram) -> u8 {
    match index {
        0..VIDEO_RAM => mbc.read(index),
        VIDEO_RAM..EXTERNAL_RAM => vram[usize::from(index - VIDEO_RAM)],
        EXTERNAL_RAM..WORK_RAM => mbc.read(index),
        WORK_RAM..ECHO_RAM => wram[usize::from(index - WORK_RAM)],
        // if greater than 0xdfff then the dma has access to a bigger echo ram than the cpu
        // from https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/hardware.rs#L215
        ECHO_RAM.. => wram[usize::from(index - ECHO_RAM)],
    }
}
