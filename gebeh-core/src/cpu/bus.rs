use crate::{
    Peripherals,
    addresses::*,
    cpu::Cpu,
    interrupts::Interrupts,
    mbc::Mbc,
    ppu::{LcdControl, LcdStatus},
    serial::SerialControl,
};

impl Cpu {
    pub fn write<M: Mbc + ?Sized>(
        &mut self,
        index: u16,
        value: u8,
        peripherals: &mut Peripherals<M>,
        _: u64,
    ) {
        match index {
            0..VIDEO_RAM => peripherals.mbc.write(index, value),
            VIDEO_RAM..EXTERNAL_RAM => {
                if peripherals.ppu.get_ppu_mode() != LcdStatus::DRAWING {
                    peripherals.ppu.get_vram_mut()[usize::from(index - VIDEO_RAM)] = value
                }
            }
            EXTERNAL_RAM..WORK_RAM => peripherals.mbc.write(index, value),
            WORK_RAM..ECHO_RAM => peripherals.wram[usize::from(index - WORK_RAM)] = value,
            ECHO_RAM..OAM => peripherals.wram[usize::from(index - ECHO_RAM)] = value,
            OAM..NOT_USABLE => {
                let ppu = peripherals.ppu.get_ppu_mode();
                if ppu != LcdStatus::DRAWING
                    && ppu != LcdStatus::OAM_SCAN
                    && !peripherals.dma.is_active()
                {
                    peripherals.ppu.get_oam_mut()[usize::from(index - OAM)] = value
                }
            }
            NOT_USABLE..JOYPAD => {}
            JOYPAD => peripherals.joypad.set_register(value),
            SB => peripherals.serial.sb = value,
            SC => peripherals
                .serial
                .set_control(SerialControl::from_bits_truncate(value)),
            0xff03 => {}
            // Citation:
            // Writing any value to this register resets it to $00
            DIV => peripherals.timer.reset_system_counter(),
            TIMER_COUNTER => peripherals.timer.set_tima(value),
            TIMER_MODULO => peripherals.timer.set_tma(value),
            TIMER_CONTROL => peripherals.timer.set_tac(value),
            0xff08..INTERRUPT_FLAG => {}
            INTERRUPT_FLAG => *peripherals.interrupts = Interrupts::from_bits_truncate(value),
            CH1_SWEEP..LCD_CONTROL => peripherals.apu.write(index, value),
            LCD_CONTROL => peripherals
                .ppu
                .set_lcd_control(LcdControl::from_bits_truncate(value)),
            // https://gbdev.io/pandocs/STAT.html#ff41--stat-lcd-status 3 last bits readonly
            LCD_STATUS => peripherals.ppu.set_interrupt_part_lcd_status(value),
            SCY => peripherals.ppu.set_scy(value),
            SCX => peripherals.ppu.set_scx(value),
            LY => {} // read only
            LYC => peripherals.ppu.lyc = value,
            DMA => {
                let dma = &mut *peripherals.dma;
                dma.dma_register = value;
                dma.dma_request = true;
            }
            BGP => peripherals.ppu.set_bgp(value),
            OBP0 => peripherals.ppu.set_obp0(value),
            OBP1 => peripherals.ppu.set_obp1(value),
            WY => peripherals.ppu.set_wy(value),
            WX => peripherals.ppu.set_wx(value),
            0xff4c => {}
            0xff4d => {}
            0xff4e => {}
            0xff4f => {}
            BOOT_ROM_MAPPING_CONTROL => self.boot_rom_mapping_control |= value != 0,
            0xff51..HRAM => {}
            HRAM..INTERRUPT_ENABLE => self.hram[usize::from(index - HRAM)] = value,
            INTERRUPT_ENABLE => self.interrupt_enable = Interrupts::from_bits_retain(value),
        }
    }
}
