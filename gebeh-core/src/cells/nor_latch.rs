#[derive(Default, Clone)]
pub struct NorLatch(bool);

impl NorLatch {
    pub fn update(&mut self, s: bool, r: bool) -> bool {
        self.0 = self.0 && !r || s;
        self.0
    }
}
