pub struct Dffr {
    clk: bool,
    pub state: bool,
}

impl Dffr {
    pub fn update(&mut self, d: bool, clk: bool, r_n: bool) -> bool {
        if !self.clk && clk {
            self.state = d;
        }
        self.state &= r_n;
        self.clk = clk;
        self.state
    }
}

pub struct DffrToggle {
    clk: bool,
    pub state: bool,
}

impl DffrToggle {
    pub fn update(&mut self, clk: bool, r_n: bool) -> bool {
        if !self.clk && clk {
            self.state = !self.state;
        }
        self.state &= r_n;
        self.clk = clk;
        self.state
    }
}
