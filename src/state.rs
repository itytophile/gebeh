pub struct State {
    pub memory: [u8; 0x10000],
}

impl Default for State {
    fn default() -> Self {
        Self {
            memory: [0; 0x10000],
        }
    }
}

pub struct WriteOnlyState<'a>(&'a mut State);

impl<'a> WriteOnlyState<'a> {
    pub fn new(state: &'a mut State) -> Self {
        Self(state)
    }
    pub fn reborrow<'c>(&'c mut self) -> WriteOnlyState<'c>
    where
        'a: 'c,
    {
        WriteOnlyState(&mut *self.0)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        println!("Writing at ${address:04x} with value 0x{value:02x}");
        self.0.memory[usize::from(address)] = value;
    }
}
