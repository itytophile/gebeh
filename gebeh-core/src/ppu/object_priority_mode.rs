pub trait ObjectPriorityModeRegs {
    fn read(&self) -> u8;
    fn write(&mut self, value: u8);
}

impl ObjectPriorityModeRegs for () {
    fn read(&self) -> u8 {
        0xff
    }

    fn write(&mut self, _: u8) {}
}

pub struct ObjectPriorityMode {
    is_dmg_style: bool,
}

impl ObjectPriorityModeRegs for ObjectPriorityMode {
    fn read(&self) -> u8 {
        self.is_dmg_style as u8 | 0b1111_1110
    }

    fn write(&mut self, value: u8) {
        self.is_dmg_style = value & 1 != 0;
    }
}
