// https://gbdev.io/pandocs/CGB_Registers.html?highlight=double#lcd-vram-dma-transfers

use crate::{Wram, external_bus::external_bus_read, mbc::Mbc, ppu::Vram};

#[derive(Clone)]
struct CopyCursor {
    src: u16,
    dst: u16,
}

#[derive(Clone)]
enum HdmaState {
    Inactive,
    GeneralPurpose(CopyCursor),
    HBlank(CopyCursor),
}

pub struct Hdma {
    source_address: u16,
    destination_address: u16,
    transfer_length: u8,
    length: u16,
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
        // Citation: the lower 7 bits of which specify the Transfer Length (divided by $10, minus 1)
        self.length = (u16::from(value & 0x7f) + 1) << 4;
        match (&self.state, value >> 7) {
            (HdmaState::Inactive, 0) => {
                self.state = HdmaState::GeneralPurpose(CopyCursor {
                    src: self.source_address,
                    dst: self.destination_address,
                })
            }
            (HdmaState::Inactive, 1) => {
                self.state = HdmaState::HBlank(CopyCursor {
                    src: self.source_address,
                    dst: self.destination_address,
                })
            }
            (HdmaState::GeneralPurpose(_), _) => panic!("The CPU is supposed to be halted"),
            // Citation: It is also possible to terminate an active HBlank transfer by writing zero to Bit 7 of FF55
            (HdmaState::HBlank(_), 0) => self.state = HdmaState::Inactive,
            _ => {}
        };
    }

    pub fn read_mode_and_length(&self) -> u8 {
        // Citation: Reading Bit 7 of FF55 can be used to confirm if the DMA transfer is active (1=Not Active, 0=Active).
        let last_bit = match self.state {
            HdmaState::Inactive => 1,
            HdmaState::GeneralPurpose(_) => panic!("The CPU is supposed to be halted"),
            HdmaState::HBlank(_) => 0,
        };

        (u8::try_from((self.length >> 4).wrapping_sub(1)).unwrap() & 0x7f) | (last_bit << 7)
    }

    // Citation: In both Normal Speed and Double Speed Mode it takes about 8 μs to transfer a block of $10 bytes.
    // That is, 8 M-cycles in Normal Speed Mode, and 16 “fast” M-cycles in Double Speed Mode.
    pub fn execute<M: Mbc + ?Sized>(&mut self, vram: &mut Vram, mbc: &M, wram: &Wram) {
        let (HdmaState::GeneralPurpose(cursor) | HdmaState::HBlank(cursor)) = &mut self.state
        else {
            return;
        };
        vram[usize::from(cursor.dst)] = external_bus_read(cursor.src, mbc, None, wram);
        vram[usize::from(cursor.dst.wrapping_add(1))] =
            external_bus_read(cursor.src.wrapping_add(1), mbc, None, wram);
        cursor.src = cursor.src.wrapping_add(2);
        cursor.dst = cursor.dst.wrapping_add(2);
        self.length = self.length.wrapping_sub(1);
        if self.length == 0 {
            self.state = HdmaState::Inactive;
        }
    }
}
