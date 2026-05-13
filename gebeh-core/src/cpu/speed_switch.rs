// https://gbdev.io/pandocs/CGB_Registers.html#ff4d--key1spd-cgb-mode-only-prepare-speed-switch

trait SpeedSwitchRegs {
    fn write_value(&mut self, value: u8);
    fn read_value(&self) -> u8;
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct SpeedSwitch: u8 {
        const DOUBLE_SPEED = 1 << 7;
        const ARMED = 1;
    }
}

impl SpeedSwitchRegs for () {
    fn write_value(&mut self, _: u8) {}

    fn read_value(&self) -> u8 {
        0xff
    }
}

impl SpeedSwitchRegs for SpeedSwitch {
    fn write_value(&mut self, value: u8) {
        self.set(SpeedSwitch::ARMED, value & 1 != 0);
    }

    fn read_value(&self) -> u8 {
        self.bits() | 0b0111_1110
    }
}
