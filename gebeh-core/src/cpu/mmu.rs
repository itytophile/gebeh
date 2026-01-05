use crate::{
    apu::Apu, cpu::Cpu, joypad::Joypad, mbc::Mbc, ppu::LcdControl, state::*, timer::Timer,
};

pub struct Peripherals<'a, M: Mbc + ?Sized> {
    pub mbc: &'a mut M,
    pub timer: &'a mut Timer,
    pub joypad: &'a mut Joypad,
    pub apu: &'a mut Apu,
}

impl<M: Mbc + ?Sized> Peripherals<'_, M> {
    pub fn get_ref(&self) -> PeripheralsRef<'_, M> {
        PeripheralsRef {
            mbc: self.mbc,
            timer: self.timer,
            joypad: self.joypad,
            apu: self.apu,
        }
    }
}

pub struct PeripheralsRef<'a, M: Mbc + ?Sized> {
    pub mbc: &'a M,
    pub timer: &'a Timer,
    pub joypad: &'a Joypad,
    pub apu: &'a Apu,
}

pub trait MmuCpuExt {
    fn read<M: Mbc + ?Sized>(
        &self,
        index: u16,
        cycles: u64,
        cpu: &Cpu,
        peripherals: PeripheralsRef<M>,
    ) -> u8;
    fn write<M: Mbc + ?Sized>(
        &mut self,
        index: u16,
        value: u8,
        cycles: u64,
        cpu: &mut Cpu,
        peripherals: Peripherals<M>,
    );
}

impl MmuCpuExt for State {
    fn read<M: Mbc + ?Sized>(
        &self,
        index: u16,
        _: u64,
        cpu: &Cpu,
        peripherals: PeripheralsRef<M>,
    ) -> u8 {
        match index {
            // https://gbdev.io/pandocs/Power_Up_Sequence.html#power-up-sequence
            ..0x100 if !cpu.boot_rom_mapping_control => cpu.boot_rom[usize::from(index)],
            ..OAM => MmuExt::read(self, index, peripherals.mbc),
            OAM..NOT_USABLE => {
                let ppu = self.lcd_status & LcdStatus::PPU_MASK;
                if ppu == LcdStatus::DRAWING || ppu == LcdStatus::OAM_SCAN || self.is_dma_active {
                    0xff
                } else {
                    self.oam[usize::from(index - OAM)]
                }
            }
            JOYPAD => peripherals.joypad.get_register(),
            SB => self.sb,
            SC => self.sc.bits() | 0b01111110,
            0xff03 => 0xff,
            DIV => peripherals.timer.get_div(),
            TIMER_COUNTER => peripherals.timer.get_tima(),
            TIMER_MODULO => peripherals.timer.get_tma(),
            TIMER_CONTROL => peripherals.timer.get_tac(),
            0xff08..INTERRUPT_FLAG => 0xff,
            INTERRUPT_FLAG => self.interrupt_flag.bits() | 0b11100000,
            CH1_SWEEP => peripherals.apu.ch1.get_nr10(),
            CH1_LENGTH_TIMER_AND_DUTY_CYCLE => {
                log::warn!(
                    "Reading ch1 length timer: {}",
                    peripherals.apu.ch1.get_nrx1()
                );
                peripherals.apu.ch1.get_nrx1()
            }
            CH1_VOLUME_AND_ENVELOPE => {
                log::warn!(
                    "Reading ch1 volume and envelope: {}",
                    peripherals.apu.ch1.get_nrx2()
                );
                peripherals.apu.ch1.get_nrx2()
            }
            CH1_PERIOD_LOW => {
                log::warn!("Reading ch1 period low: {}", peripherals.apu.ch1.get_nrx3());
                peripherals.apu.ch1.get_nrx3()
            }
            CH1_PERIOD_HIGH_AND_CONTROL => {
                log::warn!("Reading ch1 control: {}", peripherals.apu.ch1.get_nrx4());
                peripherals.apu.ch1.get_nrx4()
            }
            0xff15 => 0xff,
            CH2_LENGTH_TIMER_AND_DUTY_CYCLE => peripherals.apu.ch2.get_nrx1(),
            CH2_VOLUME_AND_ENVELOPE => peripherals.apu.ch2.get_nrx2(),
            CH2_PERIOD_LOW => peripherals.apu.ch2.get_nrx3(),
            CH2_PERIOD_HIGH_AND_CONTROL => peripherals.apu.ch2.get_nrx4(),
            CH3_DAC_ENABLE => self.ch3_dac_enable | 0b01111111,
            CH3_LENGTH_TIMER => 0xff,
            CH3_OUTPUT_LEVEL => self.ch3_output_level | 0b10011111,
            CH3_PERIOD_HIGH_AND_CONTROL => self.ch3_period_high_and_control | 0b10111111,
            CH3_PERIOD_LOW => 0xff,
            0xff1f => 0xff,
            CH4_LENGTH_TIMER => peripherals.apu.ch4.read_nr41(),
            CH4_VOLUME_AND_ENVELOPE => peripherals.apu.ch4.read_nr42(),
            CH4_FREQUENCY_AND_RANDOMNESS => peripherals.apu.ch4.read_nr43(),
            CH4_CONTROL => peripherals.apu.ch4.read_nr44(),
            MASTER_VOLUME_AND_VIN_PANNING => self.master_volume_and_vin_panning,
            SOUND_PANNING => peripherals.apu.get_nr51(),
            AUDIO_MASTER_CONTROL => peripherals.apu.get_nr52(),
            0xff27..WAVE => 0xff,
            LCD_CONTROL => self.lcd_control.bits(),
            LCD_STATUS => self.lcd_status.bits() | 0b10000000,
            SCY => self.scy,
            SCX => self.scx,
            LY => self.ly,
            LYC => self.lyc,
            DMA => self.dma_register,
            BGP => self.bgp_register,
            OBP0 => self.obp0,
            OBP1 => self.obp1,
            WY => self.wy,
            WX => self.wx,
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
        _: u64,
        cpu: &mut Cpu,
        peripherals: Peripherals<M>,
    ) {
        if self.is_dma_active && (OAM..NOT_USABLE).contains(&index) {
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
            SB => self.sb = value,
            SC => self.sc = SerialControl::from_bits_truncate(value),
            0xff03 => {}
            // Citation:
            // Writing any value to this register resets it to $00
            DIV => peripherals.timer.reset_system_counter(),
            TIMER_COUNTER => peripherals.timer.set_tima(value),
            TIMER_MODULO => peripherals.timer.set_tma(value),
            TIMER_CONTROL => peripherals.timer.set_tac(value),
            0xff08..INTERRUPT_FLAG => {}
            INTERRUPT_FLAG => self.interrupt_flag = Interruptions::from_bits_truncate(value),
            CH1_SWEEP => {
                log::warn!("Setting ch1 weep with: 0b{value:08b}");
                peripherals.apu.ch1.write_nr10(value)
            }
            CH1_LENGTH_TIMER_AND_DUTY_CYCLE => {
                log::warn!("Setting ch1 length timer with: 0b{value:08b}");
                peripherals.apu.ch1.write_nrx1(value)
            }
            CH1_VOLUME_AND_ENVELOPE => {
                log::warn!("Setting ch1 volume and envelope with: 0b{value:08b}");
                peripherals.apu.ch1.write_nrx2(value)
            }
            CH1_PERIOD_LOW => {
                log::warn!("Setting ch1 period low with: 0b{value:08b}");
                peripherals.apu.ch1.write_nrx3(value)
            }
            CH1_PERIOD_HIGH_AND_CONTROL => {
                log::warn!("Setting ch1 control with: 0b{value:08b}");
                peripherals.apu.ch1.write_nrx4(value)
            }
            0xff15 => {}
            CH2_LENGTH_TIMER_AND_DUTY_CYCLE => peripherals.apu.ch2.write_nrx1(value),
            CH2_VOLUME_AND_ENVELOPE => peripherals.apu.ch2.write_nrx2(value),
            CH2_PERIOD_LOW => peripherals.apu.ch2.write_nrx3(value),
            CH2_PERIOD_HIGH_AND_CONTROL => peripherals.apu.ch2.write_nrx4(value),
            CH3_DAC_ENABLE => self.ch3_dac_enable = value,
            CH3_LENGTH_TIMER => self.ch3_length_timer = value,
            CH3_OUTPUT_LEVEL => self.ch3_output_level = value,
            CH3_PERIOD_HIGH_AND_CONTROL => self.ch3_period_high_and_control = value,
            CH3_PERIOD_LOW => self.ch3_period_low = value,
            0xff1f => {}
            CH4_LENGTH_TIMER => peripherals.apu.ch4.write_nr41(value),
            CH4_VOLUME_AND_ENVELOPE => peripherals.apu.ch4.write_nr42(value),
            CH4_FREQUENCY_AND_RANDOMNESS => peripherals.apu.ch4.write_nr43(value),
            CH4_CONTROL => peripherals.apu.ch4.write_nr44(value),
            MASTER_VOLUME_AND_VIN_PANNING => self.master_volume_and_vin_panning = value,
            SOUND_PANNING => peripherals.apu.write_nr51(value),
            AUDIO_MASTER_CONTROL => {
                log::warn!("Writing master control with 0bx{value:08b}");
                peripherals.apu.write_nr52(value)
            }
            0xff27..WAVE => {}
            WAVE..LCD_CONTROL => {
                // TODO wave ram
            }
            LCD_CONTROL => self.lcd_control = LcdControl::from_bits_truncate(value),
            // https://gbdev.io/pandocs/STAT.html#ff41--stat-lcd-status 3 last bits readonly
            LCD_STATUS => self.set_interrupt_part_lcd_status(value),
            SCY => {
                // log::warn!("{cycles}: setting scy with {value}");
                self.scy = value
            }
            SCX => {
                // log::warn!("{cycles}: setting scx with {value}");
                self.scx = value
            }
            LY => {} // read only
            LYC => self.lyc = value,
            DMA => {
                self.dma_register = value;
                self.dma_request = true;
            }
            BGP => self.bgp_register = value,
            OBP0 => self.obp0 = value,
            OBP1 => self.obp1 = value,
            WY => self.wy = value,
            WX => self.wx = value,
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
