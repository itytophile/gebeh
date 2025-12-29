use core::ops::Deref;

// this trait will make people able to build alien MBCs
// It will be used like (&dyn Mbc) to avoid allocating too much on the stack
// if the program doesn't need MBCs with big ram.
// Don't want to do static dispatch to avoid monomorphization
pub trait Mbc {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

impl<T: Deref<Target = [u8]>> Mbc for T {
    fn read(&self, address: u16) -> u8 {
        self[usize::from(address)]
    }

    fn write(&mut self, _: u16, _: u8) {}
}
