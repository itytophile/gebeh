bitflags::bitflags! {
    #[derive(Debug, Clone, Copy,  PartialEq, Eq)]
    struct JoypadFlags: u8 {
        const NOT_BUTTONS = 1 << 5;
        const NOT_DPAD = 1 << 4;
        const NOT_START_DOWN = 1 << 3;
        const NOT_SELECT_UP = 1 << 2;
        const NOT_B_LEFT = 1 << 1;
        const NOT_A_RIGHT = 1;
    }
}

#[derive(Clone, Default)]
pub struct JoypadInput {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
}

#[derive(Clone, Default)]
pub struct Joypad {
    pub input: JoypadInput,
    is_dpad_selected: bool,
    is_buttons_selected: bool,
}

impl Joypad {
    pub fn set_register(&mut self, value: u8) {
        let value = JoypadFlags::from_bits_retain(value);
        self.is_dpad_selected = !value.contains(JoypadFlags::NOT_DPAD);
        self.is_buttons_selected = !value.contains(JoypadFlags::NOT_BUTTONS);
    }
    pub fn get_register(&self) -> u8 {
        let mut value = JoypadFlags::empty()
            | JoypadFlags::NOT_START_DOWN
            | JoypadFlags::NOT_SELECT_UP
            | JoypadFlags::NOT_B_LEFT
            | JoypadFlags::NOT_A_RIGHT;
        if self.is_dpad_selected {
            if self.input.down {
                value.remove(JoypadFlags::NOT_START_DOWN);
            }
            if self.input.up {
                value.remove(JoypadFlags::NOT_SELECT_UP);
            }
            if self.input.left {
                value.remove(JoypadFlags::NOT_B_LEFT);
            }
            if self.input.right {
                value.remove(JoypadFlags::NOT_A_RIGHT);
            }
        }
        if self.is_buttons_selected {
            if self.input.start {
                value.remove(JoypadFlags::NOT_START_DOWN);
            }
            if self.input.select {
                value.remove(JoypadFlags::NOT_SELECT_UP);
            }
            if self.input.b {
                value.remove(JoypadFlags::NOT_B_LEFT);
            }
            if self.input.a {
                value.remove(JoypadFlags::NOT_A_RIGHT);
            }
        }
        value.set(JoypadFlags::NOT_DPAD, !self.is_dpad_selected);
        value.set(JoypadFlags::NOT_BUTTONS, !self.is_buttons_selected);
        value.bits() | 0b11000000 // unused bits return 1
    }
}
