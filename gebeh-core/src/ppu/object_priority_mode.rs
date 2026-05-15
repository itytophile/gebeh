// https://gbdev.io/pandocs/CGB_Registers.html#ff6c--opri-cgb-mode-only-object-priority-mode

pub trait ObjectPriorityModeRegs: Default + Clone + Send + Sync {
    fn read(&self) -> u8;
    fn write(&mut self, value: u8);
}

impl ObjectPriorityModeRegs for () {
    fn read(&self) -> u8 {
        0xff
    }

    fn write(&mut self, _: u8) {}
}

#[derive(Default, Clone)]
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

impl ObjectPriorityMode {
    pub fn is_dmg_style(&self) -> bool {
        self.is_dmg_style
    }
}
