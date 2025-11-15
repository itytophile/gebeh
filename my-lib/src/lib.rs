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
}
