// https://iceboy.a-singer.de/doc/dmg_cells.html#tffnl
// can contain 8 Tffnl
#[derive(Default, Clone)]
pub struct Tffnl {
    state: u8,
    tclk_n: u8,
}

impl Tffnl {
    pub fn update(&mut self, index: u8, d: bool, load: bool, tclk_n: bool) -> bool {
        if load {
            self.state = set_bit_at(self.state, index, d);
        }

        if get_bit_at(self.tclk_n, index) && !tclk_n {
            self.state = set_bit_at(self.state, index, !get_bit_at(self.state, index));
        }

        self.tclk_n = set_bit_at(self.tclk_n, index, tclk_n);

        get_bit_at(self.state, index)
    }

    pub fn get_state(&self) -> u8 {
        self.state
    }
}

fn set_bit_at(byte: u8, index: u8, new_bit: bool) -> u8 {
    if new_bit {
        byte | (1 << index)
    } else {
        byte & !(1 << index)
    }
}

fn get_bit_at(byte: u8, index: u8) -> bool {
    (byte & (1 << index)) != 0
}
