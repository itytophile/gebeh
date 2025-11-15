/// The width of the VRAM.
pub const VRAM_WIDTH: u8 = 160;

/// The height of the VRAM.
pub const VRAM_HEIGHT: u8 = 144;

/// Represents a key of the joypad.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// Cursor right key.
    Right,
    /// Cursor left key.
    Left,
    /// Cursor up key.
    Up,
    /// Cursor down key.
    Down,
    /// A key.
    A,
    /// B key.
    B,
    /// Select key.
    Select,
    /// Start key.
    Start,
}

bitflags::bitflags! {
    /// The flags order is important. We can easily change the joypad state with bit operations.
    #[derive(Debug, Clone, Copy,  PartialEq, Eq, Default)]
    pub struct JoypadInput: u8 {
        const A = 1;
        const B = 1 << 1;
        const SELECT = 1 << 2;
        const START = 1 << 3;
        const RIGHT = 1 << 4;
        const LEFT = 1 << 5;
        const UP = 1 << 6;
        const DOWN = 1 << 7;
    }
}

/// Sound wave stream which generates the wave to be played by the sound device.
pub trait Stream: Send + 'static {
    /// The maximum value of the amplitude returned by this stream.
    fn max(&self) -> u16;

    /// The argument takes the sample rate, and the return value indicates the amplitude,
    /// whose max value is determined by [`Stream::max`][].
    fn next(&mut self, rate: u32) -> u16;

    /// Indicate the stream is active.
    fn on(&self) -> bool;
}

/// The interface to abstracts the OS-specific functions.
///
/// The users of this emulator library need to implement this trait,
/// providing OS-specific functions.
pub trait Clock {
    /// Clock source used by the emulator.
    /// The return value needs to be epoch time in microseconds.
    fn clock(&self) -> u64;
}
