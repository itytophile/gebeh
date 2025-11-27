pub use macros::HeapSize;

pub struct WriteOnce<'a, T>(&'a mut T, bool);

impl<'a, T> WriteOnce<'a, T> {
    pub fn new(value: &'a mut T) -> Self {
        Self(value, false)
    }
    pub fn get_mut(&mut self) -> &mut T {
        if self.1 {
            panic!("The value has already been written over")
        }
        self.1 = true;
        &mut *self.0
    }
    pub fn get_ref(&self) -> &T {
        self.0
    }
}

impl<'a, T: Copy> WriteOnce<'a, T> {
    pub fn get(&self) -> T {
        *self.0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bad_add() {
        let start = 0xc0;
        for b in 0..8 {
            for (index, r) in ['B', 'C', 'D', 'E', 'H', 'L', 'A'].into_iter().enumerate() {
                let index = if r == 'A' { index + 1 } else { index };
                println!(
                    "0x{:02x} => set_b_r({b}, {r}),",
                    start + index + (b / 2) * 0x10 + if b % 2 == 1 { 0x8 } else { 0 }
                );
            }
        }
    }
}
