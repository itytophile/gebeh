mod dffr;
mod drlatch_ee;
mod nor_latch;
mod tffnl;

pub use dffr::*;
pub use drlatch_ee::*;
pub use nor_latch::*;
pub use tffnl::*;

#[derive(Default, Clone)]
pub struct NegativeEdge(bool);

impl NegativeEdge {
    pub fn update(&mut self, input: bool) -> bool {
        let output = self.0 && !input;
        self.0 = input;
        output
    }
}
