// https://gbdev.io/pandocs/Palettes.html#lcd-color-palettes-cgb-only

#[derive(Clone)]
pub struct InnerColorPalettes {
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

const COLOR_DEPTH: usize = 5;

impl InnerColorPalettes {
    fn is_auto_increment(&self) -> bool {
        self.spec & 0x80 != 0
    }

    fn get_address(&self) -> u8 {
        self.spec & 0x3f
    }

    pub fn read_spec(&self) -> u8 {
        self.spec | 0b0100_0000
    }

    pub fn write_spec(&mut self, value: u8) {
        self.spec = value;
    }

    pub fn read_data(&self) -> u8 {
        self.data[usize::from(self.get_address())]
    }

    pub fn write_data(&mut self, value: u8) {
        let address = self.get_address();
        self.data[usize::from(address)] = value;
        if self.is_auto_increment() {
            self.spec = (self.spec & 0xc0) | ((address.wrapping_add(1)) & 0x3f);
        }
    }

    pub fn get_palette(&self, index: u8) -> [u16; 4] {
        let bytes = self.data.as_chunks::<8>().0[usize::from(index)];
        [
            u16::from_be_bytes([bytes[0], bytes[1]]),
            u16::from_be_bytes([bytes[2], bytes[3]]),
            u16::from_be_bytes([bytes[4], bytes[5]]),
            u16::from_be_bytes([bytes[6], bytes[7]]),
        ]
    }
}

#[derive(Default, Clone)]
pub struct ColorPalettes {
    pub background: InnerColorPalettes,
    pub objects: InnerColorPalettes,
}
