mod tffnl;
mod dffr;
mod nor_latch;
mod drlatch_ee;

pub use tffnl::*;
pub use dffr::*;
pub use nor_latch::*;
pub use drlatch_ee::*;

pub struct NegativeEdge(bool);

impl NegativeEdge {
    pub fn update(&mut self, input: bool) -> bool {
        let output = self.0 && !input;
        self.0 = input;
        output
    }
}
