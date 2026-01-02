#[derive(Clone, Default)]
pub struct Apu {
    is_on: bool,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    pub struct Nr52: u8 {
        const AUDIO_ON_OFF = 1 << 7;
        const CH4_ON = 1 << 3;
        const CH3_ON = 1 << 2;
        const CH2_ON = 1 << 1;
        const CH1_ON = 1;
    }
}

impl Apu {
    pub fn get_nr52(&self) -> u8 {
        let mut flags = Nr52::empty();
        flags.set(Nr52::AUDIO_ON_OFF, self.is_on);
        flags.set(Nr52::CH4_ON, self.is_ch4_on());
        flags.set(Nr52::CH3_ON, self.is_ch3_on());
        flags.set(Nr52::CH2_ON, self.is_ch2_on());
        flags.set(Nr52::CH1_ON, self.is_ch1_on());
        flags.bits() | 0b01110000
    }
    pub fn write_nr52(&mut self, value: u8) {
        self.is_on = Nr52::from_bits_retain(value).contains(Nr52::AUDIO_ON_OFF);
        if !self.is_on {
            self.clear_all_registers();
        }
    }
    fn clear_all_registers(&mut self) {
        todo!()
    }

    fn is_ch1_on(&self) -> bool {
        todo!()
    }
    fn is_ch2_on(&self) -> bool {
        todo!()
    }
    fn is_ch3_on(&self) -> bool {
        todo!()
    }
    fn is_ch4_on(&self) -> bool {
        todo!()
    }
}
