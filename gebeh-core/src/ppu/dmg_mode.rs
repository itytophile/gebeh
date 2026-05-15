// https://gbdev.io/pandocs/CGB_Registers.html#ff6c--opri-cgb-mode-only-object-priority-mode

pub trait DmgModeRegs: Default + Clone + Send + Sync {
    fn read_priority_mode(&self) -> u8;
    fn write_priority_mode(&mut self, value: u8);
    fn read_compatibility_mode(&self) -> u8;
    fn write_compatibility_mode(&mut self, value: u8);
}

impl DmgModeRegs for () {
    fn read_priority_mode(&self) -> u8 {
        0xff
    }

    fn write_priority_mode(&mut self, _: u8) {}

    fn read_compatibility_mode(&self) -> u8 {
        0xff
    }

    fn write_compatibility_mode(&mut self, _: u8) {}
}

#[derive(Default, Clone)]
pub struct DmgMode {
    is_dmg_style: bool,
    is_dmg_compatibility_mode: bool,
}

impl DmgModeRegs for DmgMode {
    fn read_priority_mode(&self) -> u8 {
        self.is_dmg_style as u8 | 0b1111_1110
    }

    fn write_priority_mode(&mut self, value: u8) {
        self.is_dmg_style = value & 1 != 0;
    }

    fn read_compatibility_mode(&self) -> u8 {
        ((self.is_dmg_compatibility_mode as u8) << 2) | 0b1111_1011
    }

    fn write_compatibility_mode(&mut self, value: u8) {
        self.is_dmg_compatibility_mode = value & 0x04 != 0;
    }
}

impl DmgMode {
    pub fn is_dmg_style(&self) -> bool {
        self.is_dmg_style
    }

    pub fn is_dmg_compatible(&self) -> bool {
        self.is_dmg_compatibility_mode
    }
}
