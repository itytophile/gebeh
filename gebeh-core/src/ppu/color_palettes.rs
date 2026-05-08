// https://gbdev.io/pandocs/Palettes.html#lcd-color-palettes-cgb-only

#[derive(Clone)]
struct InnerColorPalettes {
    spec: u8,
    data: [u8; 64],
}

impl Default for InnerColorPalettes {
    fn default() -> Self {
        Self {
            spec: 0,
            data: [0; _],
        }
    }
}

impl InnerColorPalettes {
    fn is_auto_increment(&self) -> bool {
        self.spec & 0x80 != 0
    }

    fn get_address(&self) -> u8 {
        self.spec & 0x3f
    }

    fn read_spec(&self) -> u8 {
        self.spec | 0b0100_0000
    }

    fn write_spec(&mut self, value: u8) {
        self.spec = value;
    }

    fn read_data(&self) -> u8 {
        self.data[usize::from(self.get_address())]
    }

    fn write_data(&mut self, value: u8) {
        let address = self.get_address();
        self.data[usize::from(address)] = value;
        if self.is_auto_increment() {
            self.spec = (self.spec & 0xc0) | ((address.wrapping_add(1)) & 0x3f);
        }
    }
}

#[derive(Default, Clone)]
struct ColorPalettes {
    pub background: InnerColorPalettes,
    pub objects: InnerColorPalettes,
}
