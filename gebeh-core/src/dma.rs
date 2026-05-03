use core::ops::Range;

use crate::{
    Wram,
    external_bus::{DmaPov, ExternalBus},
    mbc::Mbc,
    ppu::Ppu,
};

// about conflicts
// https://github.com/Gekkio/mooneye-gb/issues/39#issuecomment-265953981

#[derive(Clone)]
pub struct Dma {
    range: Range<u16>,
    external_bus_lock: Option<DmaPov>,
    pub dma_register: u8,
    pub dma_request: bool,
}

impl Default for Dma {
    fn default() -> Self {
        Self {
            range: 0..0,
            external_bus_lock: None,
            dma_register: 0,
            dma_request: false,
        }
    }
}

impl Dma {
    pub fn is_active(&self) -> bool {
        self.external_bus_lock.is_some()
    }

    pub fn execute<M: Mbc + ?Sized>(
        &mut self,
        external_bus: &mut ExternalBus,
        mbc: &M,
        ppu: &mut Ppu,
        wram: &Wram,
        _: u64,
    ) {
        if let Some(address) = self.range.next() {
            let lock = self
                .external_bus_lock
                .get_or_insert_with(|| DmaPov::new(external_bus));
            ppu.get_oam_mut()[usize::from(address as u8)] =
                lock.read(external_bus, address, mbc, ppu.get_vram(), wram);
        } else if let Some(lock) = self.external_bus_lock.take() {
            lock.close(external_bus);
        }

        if self.dma_request {
            self.dma_request = false;
            // for next cycle
            self.range = u16::from_be_bytes([self.dma_register, 0])
                ..u16::from_be_bytes([self.dma_register, 0xa0]);
        }
    }
}
