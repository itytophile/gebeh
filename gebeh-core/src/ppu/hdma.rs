// https://gbdev.io/pandocs/CGB_Registers.html?highlight=double#lcd-vram-dma-transfers

#[derive(Clone, Copy)]
enum HdmaState {
    Inactive,
    GeneralPurpose,
    HBlank,
}

pub struct Hdma {
    source_address: u16,
    destination_address: u16,
    transfer_length: u8,
    length: u8,
    state: HdmaState,
}

impl Hdma {
    pub fn write_source_address_low(&mut self, value: u8) {
        let [high, _] = self.source_address.to_be_bytes();
        self.source_address = u16::from_be_bytes([high, value]);
    }

    pub fn write_source_address_high(&mut self, value: u8) {
        let [_, low] = self.source_address.to_be_bytes();
        // Citation: The four lower bits of this address will be ignored and treated as 0.
        self.source_address = u16::from_be_bytes([value, low & 0xf0]);
    }

    pub fn write_destination_address_low(&mut self, value: u8) {
        let [high, _] = self.destination_address.to_be_bytes();
        // Citation: the upper 3 bits are ignored either
        self.destination_address = u16::from_be_bytes([high & 0x1f, value]);
    }

    pub fn write_destination_address_high(&mut self, value: u8) {
        let [_, low] = self.destination_address.to_be_bytes();
        // Citation: The four lower bits of this address will be ignored and treated as 0.
        self.destination_address = u16::from_be_bytes([value, low & 0xf0]);
    }

    pub fn write_length_mode_start(&mut self, value: u8) {
        self.length = value;
        self.state = match (self.state, value >> 7) {
            (HdmaState::Inactive, 0) => HdmaState::GeneralPurpose,
            (HdmaState::Inactive, 1) => HdmaState::HBlank,
            (HdmaState::GeneralPurpose, _) => panic!("The CPU is supposed to be halted"),
            // Citation: It is also possible to terminate an active HBlank transfer by writing zero to Bit 7 of FF55
            (HdmaState::HBlank, 0) => HdmaState::Inactive,
            _ => self.state,
        };
    }

    pub fn read_mode_and_length(&self) -> u8 {
        // Citation: Reading Bit 7 of FF55 can be used to confirm if the DMA transfer is active (1=Not Active, 0=Active).
        let last_bit = match self.state {
            HdmaState::Inactive => 1,
            HdmaState::GeneralPurpose => panic!("The CPU is supposed to be halted"),
            HdmaState::HBlank => 0,
        };

        (self.length & 0x7f) | (last_bit << 7)
    }
}
