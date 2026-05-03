bitflags::bitflags! {
    #[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
    pub struct Interrupts: u8 {
        const VBLANK = 1;
        const LCD = 1 << 1;
        const TIMER = 1 << 2;
        const SERIAL = 1 << 3;
        const JOYPAD = 1 << 4;
    }
}

#[cfg(test)]
mod tests {
    use super::Interrupts;

    #[test]
    fn good_priority() {
        let ints = Interrupts::all();
        let mut ints = ints.iter();
        assert_eq!(Some(Interrupts::VBLANK), ints.next());
        assert_eq!(Some(Interrupts::LCD), ints.next());
        assert_eq!(Some(Interrupts::TIMER), ints.next());
        assert_eq!(Some(Interrupts::SERIAL), ints.next());
        assert_eq!(Some(Interrupts::JOYPAD), ints.next());
        assert_eq!(None, ints.next());
    }
}
