/// Sharable handle for I/O devices to request/cancel interrupts
#[derive(Default)]
pub struct Irq {
    pub enable: Ints,
    pub request: Ints,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct Ints: u8 {
        const VBLANK = 1;
        const LCD = 1 << 1;
        const TIMER = 1 << 2;
        const SERIAL = 1 << 3;
        const JOYPAD = 1 << 4;
    }
}
