use crate::{Ram, addresses::*, mbc::Mbc};

pub fn external_bus_read(
    index: u16,
    mbc: &(impl Mbc + ?Sized),
    vram: Option<&impl Ram>,
    wram: &impl Ram,
) -> u8 {
    match index {
        0..VIDEO_RAM => mbc.read(index),
        VIDEO_RAM..EXTERNAL_RAM => vram
            .map(|r| r[usize::from(index - VIDEO_RAM)])
            .unwrap_or(0xff),
        EXTERNAL_RAM..WORK_RAM => mbc.read(index),
        WORK_RAM..ECHO_RAM => wram[usize::from(index - WORK_RAM)],
        // if greater than 0xdfff then the dma has access to a bigger echo ram than the cpu
        // from https://github.com/Gekkio/mooneye-gb/blob/3856dcbca82a7d32bd438cc92fd9693f868e2e23/core/src/hardware.rs#L215
        ECHO_RAM.. => wram[usize::from(index - ECHO_RAM)],
    }
}
