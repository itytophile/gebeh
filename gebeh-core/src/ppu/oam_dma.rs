use core::ops::Range;

use crate::{
    Ram,
    addresses::{NOT_USABLE, OAM},
    external_bus::external_bus_read,
    mbc::Mbc,
};

// about conflicts
// https://github.com/Gekkio/mooneye-gb/issues/39#issuecomment-265953981

pub type Oam = [u8; (NOT_USABLE - OAM) as usize];

#[derive(Clone)]
pub struct OamDma {
    range: Range<u16>,
    is_active: bool,
    pub dma_register: u8,
    pub dma_request: bool,
    oam: Oam,
}

impl Default for OamDma {
    fn default() -> Self {
        Self {
            range: 0..0,
            is_active: false,
            dma_register: 0,
            dma_request: false,
            oam: [0; _],
        }
    }
}

pub const BLOCKED_OAM: Oam = [0xff; _];

impl OamDma {
    pub fn execute(
        &mut self,
        mbc: &(impl Mbc + ?Sized),
        vram: Option<&impl Ram>,
        wram: &impl Ram,
        _: u64,
    ) {
        if let Some(address) = self.range.next() {
            self.is_active = true;
            self.oam[usize::from(address as u8)] = external_bus_read(address, mbc, vram, wram);
        } else {
            self.is_active = false;
        }

        if self.dma_request {
            self.dma_request = false;
            // for next cycle
            self.range = u16::from_be_bytes([self.dma_register, 0])
                ..u16::from_be_bytes([self.dma_register, 0xa0]);
        }
    }

    pub fn trigger_dma(&mut self, value: u8) {
        self.dma_register = value;
        self.dma_request = true;
    }

    pub fn get_oam(&self) -> &Oam {
        if self.is_active {
            &BLOCKED_OAM
        } else {
            &self.oam
        }
    }

    pub fn write_oam(&mut self, index: u8, value: u8) {
        if !self.is_active {
            self.oam[usize::from(index)] = value;
        }
    }
}
