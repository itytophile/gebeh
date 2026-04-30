pub struct Dffr {
    clk: bool,
    pub state: bool,
}

impl Dffr {
    pub fn update(&mut self, d: bool, clk: bool) {
        if !self.clk && clk {
            self.state = d;
        }
        self.clk = clk;
    }
}
