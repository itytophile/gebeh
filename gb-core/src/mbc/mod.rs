// this trait will make people able to build alien MBCs
// It will be used like (&dyn Mbc) to avoid allocating too much on the stack
// if the program doesn't need MBCs with big ram.
// Don't want to do static dispatch to avoid monomorphization
pub trait Mbc {
    fn read(address: u16) -> u8;
    fn write(address: u16, value: u8);
}
