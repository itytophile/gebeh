use crate::{cpu::Cpu, mbc::Mbc, ppu::LcdControl, state::*, timer::Timer};

pub struct Peripherals<'a> {
    pub mbc: &'a mut dyn Mbc,
    pub timer: &'a mut Timer,
}

impl Peripherals<'_> {
    pub fn get_ref(&self) -> PeripheralsRef<'_> {
        PeripheralsRef {
            mbc: self.mbc,
            timer: self.timer,
        }
    }
}

pub struct PeripheralsRef<'a> {
    pub mbc: &'a dyn Mbc,
    pub timer: &'a Timer,
}

pub trait MmuCpuExt {
    fn read(&self, index: u16, cycles: u64, cpu: &Cpu, peripherals: PeripheralsRef) -> u8;
    fn write(
        &mut self,
        index: u16,
        value: u8,
        cycles: u64,
        cpu: &mut Cpu,
        peripherals: Peripherals,
    );
}

impl MmuCpuExt for State {
    fn read(&self, index: u16, _: u64, cpu: &Cpu, peripherals: PeripheralsRef) -> u8 {
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
            JOYPAD => {
                (if self
                    .joypad
                    .contains(JoypadFlags::NOT_BUTTONS | JoypadFlags::NOT_DPAD)
                {
                    // https://gbdev.io/pandocs/Joypad_Input.html#ff00--p1joyp-joypad
                    self.joypad.bits() | 0xf
                } else {
                    self.joypad.bits()
                }) | 0b11000000 // unused bits return 1
            }
            SB => self.sb,
            SC => self.sc.bits() | 0b01111110,
            0xff03 => 0xff,
            DIV => peripherals.timer.get_div(),
            TIMER_COUNTER => peripherals.timer.get_tima(),
            TIMER_MODULO => peripherals.timer.get_tma(),
            TIMER_CONTROL => peripherals.timer.get_tac(),
            0xff08..INTERRUPT_FLAG => 0xff,
            INTERRUPT_FLAG => self.interrupt_flag.bits() | 0b11100000,
            SWEEP => self.sweep | 0b10000000,
            LENGTH_TIMER_AND_DUTY_CYCLE => self.length_timer_and_duty_cycle,
            VOLUME_AND_ENVELOPE => self.volume_and_envelope,
            CHANNEL_1_PERIOD_LOW => 0xff,
            CHANNEL_1_PERIOD_HIGH_AND_CONTROL => {
                self.channel_1_period_high_and_control | 0b10111111
            }
            0xff15 => 0xff,
            CHANNEL_2_LENGTH_TIMER_AND_DUTY_CYCLE => self.channel_2_length_timer_and_duty_cycle,
            CHANNEL_2_VOLUME_AND_ENVELOPE => self.channel_2_volume_and_envelope,
            CHANNEL_2_PERIOD_LOW => 0xff,
            CHANNEL_2_PERIOD_HIGH_AND_CONTROL => {
                self.channel_2_period_high_and_control | 0b10111111
            }
            CHANNEL_3_DAC_ENABLE => self.channel_3_dac_enable | 0b01111111,
            CHANNEL_3_LENGTH_TIMER => 0xff,
            CHANNEL_3_OUTPUT_LEVEL => self.channel_3_output_level | 0b10011111,
            CHANNEL_3_PERIOD_HIGH_AND_CONTROL => {
                self.channel_3_period_high_and_control | 0b10111111
            }
            CHANNEL_3_PERIOD_LOW => 0xff,
            0xff1f => 0xff,
            CHANNEL_4_LENGTH_TIMER => 0xff,
            CHANNEL_4_VOLUME_AND_ENVELOPE => self.channel_4_volume_and_envelope,
            CHANNEL_4_FREQUENCY_AND_RANDOMNESS => self.channel_4_frequency_and_randomness,
            CHANNEL_4_CONTROL => self.channel_4_control | 0b10111111,
            MASTER_VOLUME_AND_VIN_PANNING => self.master_volume_and_vin_panning,
            SOUND_PANNING => self.sound_panning,
            AUDIO_MASTER_CONTROL => self.audio_master_control | 0b01110000,
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

    fn write(&mut self, index: u16, value: u8, _: u64, cpu: &mut Cpu, peripherals: Peripherals) {
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
            JOYPAD => {
                self.joypad
                    .remove(JoypadFlags::NOT_BUTTONS | JoypadFlags::NOT_DPAD);
                self.joypad |= JoypadFlags::from_bits_retain(value)
                    & (JoypadFlags::NOT_BUTTONS | JoypadFlags::NOT_DPAD)
            }
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
            SWEEP => self.sweep = value,
            LENGTH_TIMER_AND_DUTY_CYCLE => self.length_timer_and_duty_cycle = value,
            VOLUME_AND_ENVELOPE => self.volume_and_envelope = value,
            CHANNEL_1_PERIOD_LOW => self.channel_1_period_low = value,
            CHANNEL_1_PERIOD_HIGH_AND_CONTROL => self.channel_1_period_high_and_control = value,
            0xff15 => {}
            CHANNEL_2_LENGTH_TIMER_AND_DUTY_CYCLE => {
                self.channel_2_length_timer_and_duty_cycle = value
            }
            CHANNEL_2_VOLUME_AND_ENVELOPE => self.channel_2_volume_and_envelope = value,
            CHANNEL_2_PERIOD_LOW => self.channel_2_period_low = value,
            CHANNEL_2_PERIOD_HIGH_AND_CONTROL => self.channel_2_period_high_and_control = value,
            CHANNEL_3_DAC_ENABLE => self.channel_3_dac_enable = value,
            CHANNEL_3_LENGTH_TIMER => self.channel_3_length_timer = value,
            CHANNEL_3_OUTPUT_LEVEL => self.channel_3_output_level = value,
            CHANNEL_3_PERIOD_HIGH_AND_CONTROL => self.channel_3_period_high_and_control = value,
            CHANNEL_3_PERIOD_LOW => self.channel_3_period_low = value,
            0xff1f => {}
            CHANNEL_4_LENGTH_TIMER => self.channel_4_length_timer = value,
            CHANNEL_4_VOLUME_AND_ENVELOPE => self.channel_4_volume_and_envelope = value,
            CHANNEL_4_FREQUENCY_AND_RANDOMNESS => self.channel_4_frequency_and_randomness = value,
            CHANNEL_4_CONTROL => self.channel_4_control = value,
            MASTER_VOLUME_AND_VIN_PANNING => self.master_volume_and_vin_panning = value,
            SOUND_PANNING => self.sound_panning = value,
            AUDIO_MASTER_CONTROL => self.audio_master_control = value,
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
