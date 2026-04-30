// https://iceboy.a-singer.de/doc/dmg_cells.html#tffnl
#[derive(Default, Clone)]
pub struct Tffnl { pub state: bool, pub tclk_n: bool }

impl Tffnl {
    pub fn update(&mut self,d: bool, tclk_n: bool, load: bool) -> bool {
        if load {
            self.state = d;
        }

        if self.tclk_n && !tclk_n {
            self.state = !self.state;
        }

        self.tclk_n = tclk_n;

        self.state
    }
}
