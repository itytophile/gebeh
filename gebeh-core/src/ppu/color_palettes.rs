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

    pub fn get_color(&self, index: u8) -> u16 {
        let base = usize::from(index) * 3;
        let r = self.get_5bits(base);
        let g = self.get_5bits(base + 1);
        let b = self.get_5bits(base + 2);
        u16::from(r) | (u16::from(g) >> 5) | (u16::from(b) >> 10)
    }

    fn get_5bits(&self, five_bits_index: usize) -> u8 {
        let bit_index = five_bits_index * COLOR_DEPTH;
        let byte_index = bit_index / 8;
        let [first, second] = self.data.as_chunks::<2>().0[byte_index];
        let first = first << (bit_index % 8);
        if bit_index + COLOR_DEPTH > 8 {
            first | second >> (8 - (bit_index + COLOR_DEPTH) % 8)
        } else {
            first
        }
    }
}

#[derive(Default, Clone)]
pub struct ColorPalettes {
    pub background: InnerColorPalettes,
    pub objects: InnerColorPalettes,
}
