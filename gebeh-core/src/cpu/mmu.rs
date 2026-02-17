use crate::{
    apu::Apu,
    cpu::Cpu,
    dma::Dma,
    joypad::Joypad,
    mbc::Mbc,
    ppu::{LcdControl, Ppu},
    state::*,
    timer::Timer,
};

pub struct Peripherals<'a, M: Mbc + ?Sized> {
    pub mbc: &'a mut M,
    pub timer: &'a mut Timer,
    pub joypad: &'a mut Joypad,
    pub apu: &'a mut Apu,
    pub ppu: &'a mut Ppu,
    pub dma: &'a mut Dma,
}

impl<M: Mbc + ?Sized> Peripherals<'_, M> {
    pub fn get_ref(&self) -> PeripheralsRef<'_, M> {
        PeripheralsRef {
            mbc: self.mbc,
            timer: self.timer,
            joypad: self.joypad,
            apu: self.apu,
            ppu: self.ppu,
            dma: self.dma,
        }
    }
}

pub struct PeripheralsRef<'a, M: Mbc + ?Sized> {
    pub mbc: &'a M,
    pub timer: &'a Timer,
    pub joypad: &'a Joypad,
    pub apu: &'a Apu,
    pub ppu: &'a Ppu,
    pub dma: &'a Dma,
}

pub trait MmuCpuExt {
    fn read<M: Mbc + ?Sized>(
        &self,
        index: u16,
        cpu: &Cpu,
        peripherals: PeripheralsRef<M>,
        cycles: u64,
    ) -> u8;
    fn write<M: Mbc + ?Sized>(
        &mut self,
        index: u16,
        value: u8,
        cpu: &mut Cpu,
        peripherals: &mut Peripherals<M>,
        cycles: u64,
    );
}

impl MmuCpuExt for State {
    fn read<M: Mbc + ?Sized>(
        &self,
        index: u16,
        cpu: &Cpu,
        peripherals: PeripheralsRef<M>,
        cycles: u64,
    ) -> u8 {
        match index {
            // https://gbdev.io/pandocs/Power_Up_Sequence.html#power-up-sequence
            ..0x100 if !cpu.boot_rom_mapping_control => cpu.boot_rom[usize::from(index)],
            ..OAM => MmuExt::read(self, index, peripherals.mbc),
            OAM..NOT_USABLE => {
                let ppu = self.lcd_status & LcdStatus::PPU_MASK;
                if ppu == LcdStatus::DRAWING
                    || ppu == LcdStatus::OAM_SCAN
                    || peripherals.dma.is_active()
                {
                    0xff
                } else {
                    self.oam[usize::from(index - OAM)]
                }
            }
            JOYPAD => peripherals.joypad.get_register(),
            SB => {
                log::info!("Reading sb {}", self.sb);
                self.sb
            }
            SC => {
                log::info!("Reading sc {:?}", self.sc);
                self.sc.bits() | 0b01111110
            }
            0xff03 => 0xff,
            DIV => peripherals.timer.get_div(),
            TIMER_COUNTER => peripherals.timer.get_tima(),
            TIMER_MODULO => peripherals.timer.get_tma(),
            TIMER_CONTROL => peripherals.timer.get_tac(),
            0xff08..INTERRUPT_FLAG => 0xff,
            INTERRUPT_FLAG => self.interrupt_flag.bits() | 0b11100000,
            CH1_SWEEP..LCD_CONTROL => peripherals.apu.read(index, cycles),
            LCD_CONTROL => peripherals.ppu.get_lcd_control().bits(),
            LCD_STATUS => self.lcd_status.bits() | 0b10000000,
            SCY => peripherals.ppu.get_scy(),
            SCX => peripherals.ppu.get_scx(),
            LY => peripherals.ppu.get_ly(),
            LYC => self.lyc,
            DMA => self.dma_register,
            BGP => peripherals.ppu.get_bgp(),
            OBP0 => self.obp0,
            OBP1 => self.obp1,
            WY => self.wy,
            WX => peripherals.ppu.get_wx(),
            0xff4c => 0xff,
            0xff4d => 0xff,
            0xff4e => 0xff,
            0xff4f => 0xff,
            BOOT_ROM_MAPPING_CONTROL => 0xff,
            0xff51..HRAM => 0xff,
            HRAM..INTERRUPT_ENABLE => cpu.hram[usize::from(index - HRAM)],
            INTERRUPT_ENABLE => cpu.interrupt_enable.bits(),
            _ => todo!("Reading ${index:04x}"),
        }
    }

    fn write<M: Mbc + ?Sized>(
        &mut self,
        index: u16,
        value: u8,
        cpu: &mut Cpu,
        peripherals: &mut Peripherals<M>,
        _: u64,
    ) {
        if peripherals.dma.is_active() && (OAM..NOT_USABLE).contains(&index) {
            return;
        }

        match index {
            0..VIDEO_RAM => peripherals.mbc.write(index, value),
            VIDEO_RAM..EXTERNAL_RAM => {
                if (self.lcd_status & LcdStatus::PPU_MASK) != LcdStatus::DRAWING {
                    self.video_ram[usize::from(index - VIDEO_RAM)] = value
                }
            }
            EXTERNAL_RAM..WORK_RAM => peripherals.mbc.write(index, value),
            WORK_RAM..ECHO_RAM => self.wram[usize::from(index - WORK_RAM)] = value,
            ECHO_RAM..OAM => self.wram[usize::from(index - ECHO_RAM)] = value,
            OAM..NOT_USABLE => {
                let ppu = self.lcd_status & LcdStatus::PPU_MASK;
                if ppu != LcdStatus::DRAWING && ppu != LcdStatus::OAM_SCAN {
                    self.oam[usize::from(index - OAM)] = value
                }
            }
            NOT_USABLE..JOYPAD => {}
            JOYPAD => peripherals.joypad.set_register(value),
            SB => {
                log::info!("Writing to sb 0x{value:02x}");
                self.sb = value
            }
            SC => {
                log::info!("Writing to sc {:?}", SerialControl::from_bits_truncate(value));
                self.sc = SerialControl::from_bits_truncate(value)
            }
            0xff03 => {}
            // Citation:
            // Writing any value to this register resets it to $00
            DIV => peripherals.timer.reset_system_counter(),
            TIMER_COUNTER => peripherals.timer.set_tima(value),
            TIMER_MODULO => peripherals.timer.set_tma(value),
            TIMER_CONTROL => peripherals.timer.set_tac(value),
            0xff08..INTERRUPT_FLAG => {}
            INTERRUPT_FLAG => self.interrupt_flag = Interruptions::from_bits_truncate(value),
            CH1_SWEEP..LCD_CONTROL => peripherals.apu.write(index, value),
            LCD_CONTROL => peripherals
                .ppu
                .set_lcd_control(LcdControl::from_bits_truncate(value)),
            // https://gbdev.io/pandocs/STAT.html#ff41--stat-lcd-status 3 last bits readonly
            LCD_STATUS => self.set_interrupt_part_lcd_status(value),
            SCY => peripherals.ppu.set_scy(value),
            SCX => peripherals.ppu.set_scx(value),
            LY => {} // read only
            LYC => self.lyc = value,
            DMA => {
                self.dma_register = value;
                self.dma_request = true;
            }
            BGP => peripherals.ppu.set_bgp(value),
            OBP0 => self.obp0 = value,
            OBP1 => self.obp1 = value,
            WY => self.wy = value,
            WX => {
                peripherals.ppu.set_wx(value);
            }
            0xff4c => {}
            0xff4d => {}
            0xff4e => {}
            0xff4f => {}
            BOOT_ROM_MAPPING_CONTROL => cpu.boot_rom_mapping_control = value & 0b1 != 0,
            0xff51..HRAM => {}
            HRAM..INTERRUPT_ENABLE => cpu.hram[usize::from(index - HRAM)] = value,
            INTERRUPT_ENABLE => cpu.interrupt_enable = Interruptions::from_bits_retain(value),
        }
    }
}
