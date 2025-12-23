use crate::{cpu::Cpu, ic::Ints, ppu::LcdControl, state::*};

pub struct MmuRead<'a>(pub &'a State);

impl MmuRead<'_> {
    pub fn read(&self, index: u16, cycle_count: u64, cpu: &Cpu) -> u8 {
        match index {
            ..OAM => CommonMmu(self.0).read(index),
            OAM..NOT_USABLE => {
                let ppu = self.0.lcd_status & LcdStatus::PPU_MASK;
                if ppu == LcdStatus::DRAWING || ppu == LcdStatus::OAM_SCAN || self.0.is_dma_active {
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
            DIV => {
                log::warn!(
                    "{cycle_count}: Reading div 0x{:04x} 0x{:02x}",
                    self.0.system_counter,
                    u8::try_from(self.0.system_counter >> 6 & 0xff).unwrap()
                );
                (self.0.system_counter >> 6 & 0xff).try_into().unwrap()
            }
            TIMER_COUNTER => self.0.timer_counter,
            TIMER_MODULO => self.0.timer_modulo,
            TIMER_CONTROL => self.0.timer_control | 0b11111000,
            0xff08..INTERRUPT_FLAG => 0xff,
            INTERRUPT_FLAG => cpu.interrupt_flag.bits() | 0b11100000,
            SWEEP => self.0.sweep | 0b10000000,
            LENGTH_TIMER_AND_DUTY_CYCLE => self.0.length_timer_and_duty_cycle,
            VOLUME_AND_ENVELOPE => self.0.volume_and_envelope,
            CHANNEL_1_PERIOD_LOW => 0xff,
            CHANNEL_1_PERIOD_HIGH_AND_CONTROL => {
                self.0.channel_1_period_high_and_control | 0b10111111
            }
            0xff15 => 0xff,
            CHANNEL_2_LENGTH_TIMER_AND_DUTY_CYCLE => self.0.channel_2_length_timer_and_duty_cycle,
            CHANNEL_2_VOLUME_AND_ENVELOPE => self.0.channel_2_volume_and_envelope,
            CHANNEL_2_PERIOD_LOW => 0xff,
            CHANNEL_2_PERIOD_HIGH_AND_CONTROL => {
                self.0.channel_2_period_high_and_control | 0b10111111
            }
            CHANNEL_3_DAC_ENABLE => self.0.channel_3_dac_enable | 0b01111111,
            CHANNEL_3_LENGTH_TIMER => 0xff,
            CHANNEL_3_OUTPUT_LEVEL => self.0.channel_3_output_level | 0b10011111,
            CHANNEL_3_PERIOD_HIGH_AND_CONTROL => {
                self.0.channel_3_period_high_and_control | 0b10111111
            }
            CHANNEL_3_PERIOD_LOW => 0xff,
            0xff1f => 0xff,
            CHANNEL_4_LENGTH_TIMER => 0xff,
            CHANNEL_4_VOLUME_AND_ENVELOPE => self.0.channel_4_volume_and_envelope,
            CHANNEL_4_FREQUENCY_AND_RANDOMNESS => self.0.channel_4_frequency_and_randomness,
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
            LY => {
                // log::warn!("{cycle_count}: Reading LY");
                self.0.ly
            }
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
            INTERRUPT_ENABLE => cpu.interrupt_enable.bits(),
            _ => todo!("Reading ${index:04x}"),
        }
    }
}

pub struct MmuWrite<'a>(pub &'a mut State);

impl MmuWrite<'_> {
    pub fn write(&mut self, index: u16, value: u8, cycle_count: u64, cpu: &mut Cpu) {
        if self.0.is_dma_active && (OAM..NOT_USABLE).contains(&index) {
            return;
        }

        match index {
            0..VIDEO_RAM => self.0.mbc.write(index, value),
            VIDEO_RAM..EXTERNAL_RAM => {
                if (self.0.lcd_status & LcdStatus::PPU_MASK) != LcdStatus::DRAWING {
                    self.0.video_ram[usize::from(index - VIDEO_RAM)] = value
                }
            }
            EXTERNAL_RAM..WORK_RAM => self.0.mbc.write(index, value),
            WORK_RAM..ECHO_RAM => self.0.wram[usize::from(index - WORK_RAM)] = value,
            ECHO_RAM..OAM => self.0.wram[usize::from(index - ECHO_RAM)] = value,
            OAM..NOT_USABLE => {
                let ppu = self.0.lcd_status & LcdStatus::PPU_MASK;
                if ppu != LcdStatus::DRAWING && ppu != LcdStatus::OAM_SCAN {
                    self.0.oam[usize::from(index - OAM)] = value
                }
            }
            NOT_USABLE..JOYPAD => {}
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
            DIV => {
                log::warn!("{cycle_count}: Writing div");
                self.0.system_counter = 0
            }
            TIMER_COUNTER => self.0.timer_counter = value,
            TIMER_MODULO => self.0.timer_modulo = value,
            TIMER_CONTROL => self.0.timer_control = value,
            0xff08..INTERRUPT_FLAG => {}
            INTERRUPT_FLAG => cpu.interrupt_flag = Ints::from_bits_truncate(value),
            SWEEP => self.0.sweep = value,
            LENGTH_TIMER_AND_DUTY_CYCLE => self.0.length_timer_and_duty_cycle = value,
            VOLUME_AND_ENVELOPE => self.0.volume_and_envelope = value,
            CHANNEL_1_PERIOD_LOW => self.0.channel_1_period_low = value,
            CHANNEL_1_PERIOD_HIGH_AND_CONTROL => self.0.channel_1_period_high_and_control = value,
            0xff15 => {}
            CHANNEL_2_LENGTH_TIMER_AND_DUTY_CYCLE => {
                self.0.channel_2_length_timer_and_duty_cycle = value
            }
            CHANNEL_2_VOLUME_AND_ENVELOPE => self.0.channel_2_volume_and_envelope = value,
            CHANNEL_2_PERIOD_LOW => self.0.channel_2_period_low = value,
            CHANNEL_2_PERIOD_HIGH_AND_CONTROL => self.0.channel_2_period_high_and_control = value,
            CHANNEL_3_DAC_ENABLE => self.0.channel_3_dac_enable = value,
            CHANNEL_3_LENGTH_TIMER => self.0.channel_3_length_timer = value,
            CHANNEL_3_OUTPUT_LEVEL => self.0.channel_3_output_level = value,
            CHANNEL_3_PERIOD_HIGH_AND_CONTROL => self.0.channel_3_period_high_and_control = value,
            CHANNEL_3_PERIOD_LOW => self.0.channel_3_period_low = value,
            0xff1f => {}
            CHANNEL_4_LENGTH_TIMER => self.0.channel_4_length_timer = value,
            CHANNEL_4_VOLUME_AND_ENVELOPE => self.0.channel_4_volume_and_envelope = value,
            CHANNEL_4_FREQUENCY_AND_RANDOMNESS => self.0.channel_4_frequency_and_randomness = value,
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
            SCX => {
                log::warn!("{cycle_count}: Setting scx to 0x{value:02x}");
                self.0.scx = value
            }
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
            0xff4d => {}
            0xff4e => {}
            0xff4f => {}
            BOOT_ROM_MAPPING_CONTROL => self.0.boot_rom_mapping_control = value,
            0xff51..HRAM => {}
            HRAM..INTERRUPT_ENABLE => self.0.hram[usize::from(index - HRAM)] = value,
            INTERRUPT_ENABLE => cpu.interrupt_enable = Ints::from_bits_retain(value),
        }
    }
}
