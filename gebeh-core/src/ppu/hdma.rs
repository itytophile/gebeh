// https://gbdev.io/pandocs/CGB_Registers.html?highlight=double#lcd-vram-dma-transfers

pub struct Hdma {
    source_address: u16,
    destination_address: u16,
    transfer_length: u8,
    // lenght/mode/start
    length_mode_start: u8,
}

impl Hdma {
    pub fn set_source_address_low(&mut self, value: u8) {
        let [high, _] = self.source_address.to_be_bytes();
        self.source_address = u16::from_be_bytes([high, value]);
    }

    pub fn set_source_address_high(&mut self, value: u8) {
        let [_, low] = self.source_address.to_be_bytes();
        // Citation: The four lower bits of this address will be ignored and treated as 0.
        self.source_address = u16::from_be_bytes([value, low & 0xf0]);
    }

    pub fn set_destination_address_low(&mut self, value: u8) {
        let [high, _] = self.destination_address.to_be_bytes();
        // Citation: the upper 3 bits are ignored either
        self.destination_address = u16::from_be_bytes([high & 0x1f, value]);
    }

    pub fn set_destination_address_high(&mut self, value: u8) {
        let [_, low] = self.destination_address.to_be_bytes();
        // Citation: The four lower bits of this address will be ignored and treated as 0.
        self.destination_address = u16::from_be_bytes([value, low & 0xf0]);
    }

    pub fn set_length_mode_start(&mut self, value: u8) {
        self.length_mode_start = value;
    }

    pub fn get_length_mode_start(&self) -> u8 {
        self.length_mode_start
    }
}
