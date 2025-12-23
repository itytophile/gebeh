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

#[cfg(test)]
mod tests {
    use crate::ic::Ints;

    #[test]
    fn good_priority() {
        let ints = Ints::all();
        let mut ints = ints.iter();
        assert_eq!(Some(Ints::VBLANK), ints.next());
        assert_eq!(Some(Ints::LCD), ints.next());
        assert_eq!(Some(Ints::TIMER), ints.next());
        assert_eq!(Some(Ints::SERIAL), ints.next());
        assert_eq!(Some(Ints::JOYPAD), ints.next());
        assert_eq!(None, ints.next());
    }
}
