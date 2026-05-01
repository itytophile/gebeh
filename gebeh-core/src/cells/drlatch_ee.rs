pub struct DrlatchEe(bool);

impl DrlatchEe {
    pub fn update(&mut self, d: bool, ena: bool, r_n: bool) -> bool {
        if ena {
            self.0 = d;
        }

        self.0 &= r_n;
        self.0
    }

    pub fn get_state(&self) -> bool {
        self.0
    }
}
