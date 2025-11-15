/// Sharable handle for I/O devices to request/cancel interrupts
#[derive(Default)]
pub struct Irq {
    enable: Ints,
    pub request: Ints,
}

impl Irq {
    fn get_first_ready_interrupt(&self) -> Option<(Ints, u8)> {
        let flag = self.enable.intersection(self.request).iter().next()?;
        let pc = match flag {
            Ints::VBLANK => 0x40,
            Ints::LCD => 0x48,
            Ints::TIMER => 0x50,
            Ints::SERIAL => 0x58,
            Ints::JOYPAD => 0x60,
            _ => return None,
        };
        Some((flag, pc))
    }

    pub fn peek(&self) -> Option<u8> {
        self.get_first_ready_interrupt().map(|(_, pc)| pc)
    }

    pub fn pop(&mut self) -> Option<u8> {
        let (flag, pc) = self.get_first_ready_interrupt()?;
        self.request -= flag;
        Some(pc)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy,  PartialEq, Eq)]
    pub struct Ints: u8 {
        const VBLANK = 1;
        const LCD = 1 << 1;
        const TIMER = 1 << 2;
        const SERIAL = 1 << 3;
        const JOYPAD = 1 << 4;
    }
}

/// Read IE register (0xffff)
pub fn read_enabled(irq: &Irq) -> u8 {
    let v = irq.enable.bits();
    println!("Read interrupt enable: {:02x}", v);
    v
}

/// Write IF register (0xff0f)
pub fn read_flags(irq: &Irq) -> u8 {
    let v = irq.request.bits();
    println!("Read interrupt: {:02x}", v);
    v | 0xe0
}

/// Write IE register (0xffff)
pub fn write_enabled(value: u8, irq: &mut Irq) {
    println!("Write interrupt enable: {:02x}", value);
    irq.enable = Ints::from_bits_retain(value);
}

/// Write IF register (0xff0f)
pub fn write_flags(value: u8, irq: &mut Irq) {
    println!("Write interrupt: {:02x}", value);
    irq.request = Ints::from_bits_retain(value);
}
