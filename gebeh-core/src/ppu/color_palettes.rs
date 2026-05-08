// https://gbdev.io/pandocs/Palettes.html#lcd-color-palettes-cgb-only

struct InnerColorPalettes {
    spec: u8,
    data: [u8; 64],
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

struct ColorPalettes {
    background: InnerColorPalettes,
    objects: InnerColorPalettes,
}

impl ColorPalettes {
    fn read_background_spec(&self) -> u8 {
        self.background.read_spec()
    }

    fn write_background_spec(&mut self, value: u8) {
        self.background.write_spec(value);
    }

    fn read_background_data(&self) -> u8 {
        self.background.read_data()
    }

    fn write_background_data(&mut self, value: u8) {
        self.background.write_data(value);
    }

    fn read_objects_spec(&self) -> u8 {
        self.objects.read_spec()
    }

    fn write_objects_spec(&mut self, value: u8) {
        self.objects.write_spec(value);
    }

    fn read_objects_data(&self) -> u8 {
        self.objects.read_data()
    }

    fn write_objects_data(&mut self, value: u8) {
        self.objects.write_data(value);
    }
}
