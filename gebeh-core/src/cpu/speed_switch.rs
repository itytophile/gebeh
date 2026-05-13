// https://gbdev.io/pandocs/CGB_Registers.html#ff4d--key1spd-cgb-mode-only-prepare-speed-switch

pub trait SpeedSwitch: Default + Clone + Send + Sync {
    fn write_value(&mut self, value: u8);
    fn read_value(&self) -> u8;
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CgbSpeedSwitch: u8 {
        const DOUBLE_SPEED = 1 << 7;
        const ARMED = 1;
    }
}

impl SpeedSwitch for () {
    fn write_value(&mut self, _: u8) {}

    fn read_value(&self) -> u8 {
        0xff
    }
}

impl SpeedSwitch for CgbSpeedSwitch {
    fn write_value(&mut self, value: u8) {
        self.set(CgbSpeedSwitch::ARMED, value & 1 != 0);
    }

    fn read_value(&self) -> u8 {
        self.bits() | 0b0111_1110
    }
}
