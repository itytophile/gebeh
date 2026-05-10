use crate::{
    Model, Peripherals, PeripheralsRef, Ram,
    addresses::*,
    cpu::Cpu,
    interrupts::Interrupts,
    mbc::Mbc,
    ppu::{LcdControl, color_palettes::ColorPalettesRegs, hdma::HdmaRegs, vram::VramRegs},
    serial::SerialControl,
    wram::Wram,
};

impl Cpu {
    pub fn internal_bus_read(
        &self,
        index: u16,
        peripherals: PeripheralsRef<impl Mbc + ?Sized, impl Model>,
        cycles: u64,
    ) -> u8 {
        match index {
            OAM..NOT_USABLE => peripherals.ppu.get_oam()[usize::from(index - OAM)],
            JOYPAD => peripherals.joypad.get_register(),
            SB => peripherals.serial.sb,
            SC => peripherals.serial.get_control().bits() | 0b01111110,
            0xff03 => 0xff,
            DIV => peripherals.timer.get_div(),
            TIMER_COUNTER => peripherals.timer.get_tima(),
            TIMER_MODULO => peripherals.timer.get_tma(),
            TIMER_CONTROL => peripherals.timer.get_tac(),
            0xff08..INTERRUPT_FLAG => 0xff,
            INTERRUPT_FLAG => peripherals.interrupts.bits() | 0b11100000,
            CH1_SWEEP..LCD_CONTROL => peripherals.apu.read(index, cycles),
            LCD_CONTROL => peripherals.ppu.get_lcd_control().bits(),
            LCD_STATUS => peripherals.ppu.get_lcd_status().bits() | 0b10000000,
            SCY => peripherals.ppu.get_scy(),
            SCX => peripherals.ppu.get_scx(),
            LY => peripherals.ppu.get_ly(),
            LYC => peripherals.ppu.lyc,
            DMA => peripherals.ppu.get_dma_register(),
            BGP => peripherals.ppu.get_bgp(),
            OBP0 => peripherals.ppu.get_obp0(),
            OBP1 => peripherals.ppu.get_obp1(),
            WY => peripherals.ppu.get_wy(),
            WX => peripherals.ppu.get_wx(),
            0xff4c => 0xff,
            0xff4d => 0xff,
            0xff4e => 0xff,
            VRAM_BANK => peripherals.ppu.get_vram().read_bank(),
            BOOT_ROM_MAPPING_CONTROL => 0xff,
            HDMA_SOURCE_HIGH..HDMA_LENGTH_AND_MODE => 0xff,
            HDMA_LENGTH_AND_MODE => peripherals.hdma.read_mode_and_length(),
            0xff56..BCPS_BGPI => 0xff,
            BCPS_BGPI => peripherals.ppu.get_color_palettes().read_background_spec(),
            BCPD_BGPD => peripherals.ppu.get_color_palettes().read_background_data(),
            OCPS_OGPI => peripherals.ppu.get_color_palettes().read_obj_spec(),
            OCPD_OGPD => peripherals.ppu.get_color_palettes().read_obj_data(),
            0xff6c..WRAM_BANK => 0xff,
            WRAM_BANK => peripherals.wram.read_bank(),
            0xFF71..HRAM => 0xff,
            HRAM..INTERRUPT_ENABLE => self.hram[usize::from(index - HRAM)],
            INTERRUPT_ENABLE => self.interrupt_enable.bits(),
            _ => todo!("Reading ${index:04x} from internal bus"),
        }
    }
    pub fn write(
        &mut self,
        index: u16,
        value: u8,
        peripherals: &mut Peripherals<impl Mbc + ?Sized, impl Model>,
        _: u64,
    ) {
        match index {
            0..VIDEO_RAM => peripherals.mbc.write(index, value),
            VIDEO_RAM..EXTERNAL_RAM => peripherals.ppu.write_vram(index - VIDEO_RAM, value),
            EXTERNAL_RAM..WORK_RAM => peripherals.mbc.write(index, value),
            WORK_RAM..ECHO_RAM => peripherals.wram.write(index - WORK_RAM, value),
            ECHO_RAM..OAM => peripherals.wram.write(index - ECHO_RAM, value),
            OAM..NOT_USABLE => peripherals
                .ppu
                .write_oam(u8::try_from(index - OAM).unwrap(), value),
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
            DMA => peripherals.ppu.trigger_dma(value),
            BGP => peripherals.ppu.set_bgp(value),
            OBP0 => peripherals.ppu.set_obp0(value),
            OBP1 => peripherals.ppu.set_obp1(value),
            WY => peripherals.ppu.set_wy(value),
            WX => peripherals.ppu.set_wx(value),
            0xff4c => {}
            0xff4d => {}
            0xff4e => {}
            VRAM_BANK => peripherals.ppu.get_vram_mut().write_bank(value),
            BOOT_ROM_MAPPING_CONTROL => self.boot_rom_mapping_control |= value != 0,
            HDMA_SOURCE_HIGH => peripherals.hdma.write_source_address_high(value),
            HDMA_SOURCE_LOW => peripherals.hdma.write_source_address_low(value),
            HDMA_DESTINATION_HIGH => peripherals.hdma.write_destination_address_high(value),
            HDMA_DESTINATION_LOW => peripherals.hdma.write_destination_address_low(value),
            HDMA_LENGTH_AND_MODE => peripherals.hdma.write_length_mode_start(value),
            0xff56..BCPS_BGPI => {}
            BCPS_BGPI => peripherals
                .ppu
                .get_color_palettes_mut()
                .write_background_spec(value),
            BCPD_BGPD => peripherals
                .ppu
                .get_color_palettes_mut()
                .write_background_data(value),
            OCPS_OGPI => peripherals
                .ppu
                .get_color_palettes_mut()
                .write_obj_spec(value),
            OCPD_OGPD => peripherals
                .ppu
                .get_color_palettes_mut()
                .write_obj_data(value),
            0xff6c..WRAM_BANK => {}
            WRAM_BANK => peripherals.wram.write_bank(value),
            0xff71..HRAM => {}
            HRAM..INTERRUPT_ENABLE => self.hram[usize::from(index - HRAM)] = value,
            INTERRUPT_ENABLE => self.interrupt_enable = Interrupts::from_bits_retain(value),
        }
    }
}
