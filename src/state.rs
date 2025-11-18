const VIDEO_RAM: u16 = 0x8000;
const EXTERNAL_RAM: u16 = 0xa000;
const WORK_RAM: u16 = 0xc000;
const ECHO_RAM: u16 = 0xe000;
const SB: u16 = 0xff01; // Serial transfer data
const SC: u16 = 0xff02; // Serial transfer control
const INTERRUPT_FLAG: u16 = 0xff0f;
const AUDIO: u16 = 0xff10;
const WAVE: u16 = 0xff30;
const LCD_CONTROL: u16 = 0xff40;
const SCY: u16 = 0xff42;
const SCX: u16 = 0xff43;
const LY: u16 = 0xff44; // LCD Y
const DMA: u16 = 0xff46;
const BGP: u16 = 0xff47;
const OBP0: u16 = 0xff48;
const OBP1: u16 = 0xff49;
const WY: u16 = 0xff4a;
const WX: u16 = 0xff4b;
const BOOT_ROM_MAPPING_CONTROL: u16 = 0xff50;
const HRAM: u16 = 0xff80;
const INTERRUPT_ENABLE: u16 = 0xffff;

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

pub struct State {
    pub boot_rom: &'static [u8; 256],
    pub rom: &'static [u8],
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
    pub audio: [u8; (WAVE - AUDIO) as usize],
    pub scy: u8,
    pub scx: u8,
    pub lcd_control: LcdControl,
    pub ly: u8,
    pub boot_rom_mapping_control: u8,
    pub sb: u8,
    pub sc: u8,
    pub wy: u8,
    pub wx: u8,
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
            audio: [0; (WAVE - AUDIO) as usize],
            scx: 0,
            scy: 0,
            lcd_control: LcdControl::empty(),
            ly: 0,
            rom,
            boot_rom_mapping_control: 0,
            sb: 0,
            sc: 0,
            wy: 0,
            wx: 0,
        }
    }
}

use crate::{gpu::LcdControl, ic::Ints};

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
    pub fn mmu(&mut self) -> MmuWrite<'_> {
        MmuWrite(self.0)
    }
    pub fn set_ie(&mut self, i: Ints) {
        self.0.interrupt_enable = i;
    }
    pub fn set_if(&mut self, i: Ints) {
        self.0.interrupt_flag = i;
    }
    pub fn set_ly(&mut self, value: u8) {
        self.0.ly = value;
    }
}

pub struct MmuRead<'a>(&'a State);

impl MmuRead<'_> {
    pub fn read(&self, index: u16) -> u8 {
        match index {
            0..VIDEO_RAM => {
                if self.0.boot_rom_mapping_control == 0 {
                    if let Some(value) = self.0.boot_rom.get(usize::from(index)).copied() {
                        value
                    } else {
                        let value = self.0.rom[usize::from(index)];
                        println!("ROM ${index:04x} => 0x{value:02x}");
                        value
                    }
                } else {
                    self.0.rom[usize::from(index)]
                }
            }
            VIDEO_RAM..EXTERNAL_RAM => self.0.video_ram[usize::from(index - VIDEO_RAM)],
            WORK_RAM..ECHO_RAM => self.0.wram[usize::from(index - WORK_RAM)],
            SB => self.0.sb,
            SC => self.0.sc,
            INTERRUPT_FLAG => self.0.interrupt_flag.bits(),
            AUDIO..WAVE => self.0.audio[usize::from(index - AUDIO)],
            LCD_CONTROL => self.0.lcd_control.bits(),
            SCY => self.0.scy,
            SCX => self.0.scx,
            LY => self.0.ly,
            DMA => self.0.dma_register,
            BGP => self.0.bgp_register,
            OBP0 => self.0.obp0,
            OBP1 => self.0.obp1,
            WY => self.0.wy,
            WX => self.0.wx,
            BOOT_ROM_MAPPING_CONTROL => self.0.boot_rom_mapping_control,
            HRAM..INTERRUPT_ENABLE => self.0.hram[usize::from(index - HRAM)],
            INTERRUPT_ENABLE => self.0.interrupt_enable.bits(),
            _ => todo!("{index:04x}"),
        }
    }
}

pub struct MmuWrite<'a>(&'a mut State);
// TODO gérer les interrupts
impl MmuWrite<'_> {
    pub fn write(&mut self, index: u16, value: u8) {
        match index {
            0..VIDEO_RAM => panic!("Trying to write to ROM"),
            VIDEO_RAM..EXTERNAL_RAM => {
                println!("VRAM ${index:04x} => 0x{value:x}");
                self.0.video_ram[usize::from(index - VIDEO_RAM)] = value
            }
            WORK_RAM..ECHO_RAM => self.0.wram[usize::from(index - WORK_RAM)] = value,
            SB => self.0.sb = value,
            SC => self.0.sc = value,
            INTERRUPT_FLAG => self.0.interrupt_flag = Ints::from_bits_retain(value),
            AUDIO..WAVE => self.0.audio[usize::from(index - AUDIO)] = value,
            LCD_CONTROL => self.0.lcd_control = LcdControl::from_bits_retain(value),
            SCY => {
                println!("SCY {value:x}");
                self.0.scy = value
            }
            SCX => self.0.scx = value,
            LY => {} // read only
            DMA => {
                self.0.dma_register = value;
                self.0.dma_request = true;
                todo!()
            }
            BGP => self.0.bgp_register = value,
            OBP0 => self.0.obp0 = value,
            OBP1 => self.0.obp1 = value,
            WY => self.0.wy = value,
            WX => self.0.wx = value,
            BOOT_ROM_MAPPING_CONTROL => self.0.boot_rom_mapping_control = value,
            HRAM..INTERRUPT_ENABLE => self.0.hram[usize::from(index - HRAM)] = value,
            INTERRUPT_ENABLE => self.0.interrupt_enable = Ints::from_bits_retain(value),
            _ => todo!("${index:04x}"),
        }
    }
}
