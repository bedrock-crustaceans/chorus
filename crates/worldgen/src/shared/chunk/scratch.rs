use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::thread::LocalKey;

pub struct Scratch<T: 'static> {
    buffer: Vec<T>,
    home: &'static LocalKey<RefCell<Vec<T>>>,
}

impl<T: Clone + 'static> Scratch<T> {
    pub fn take(home: &'static LocalKey<RefCell<Vec<T>>>, len: usize, value: T) -> Self {
        let mut buffer = home.try_with(|spare| std::mem::take(&mut *spare.borrow_mut())).unwrap_or_default();
        buffer.clear();
        buffer.resize(len, value);
        Self { buffer, home }
    }
}

impl<T> Deref for Scratch<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.buffer
    }
}

impl<T> DerefMut for Scratch<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.buffer
    }
}

impl<T> Drop for Scratch<T> {
    fn drop(&mut self) {
        let buffer = std::mem::take(&mut self.buffer);
        let _ = self.home.try_with(|spare| *spare.borrow_mut() = buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static SPARE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }

    #[test]
    fn a_taken_buffer_is_filled_to_the_requested_length() {
        let buffer = Scratch::take(&SPARE, 5, 7u32);
        assert_eq!(&*buffer, &[7, 7, 7, 7, 7]);
        drop(buffer);
        drop(Scratch::take(&SPARE, 10, 0u32));
        assert_eq!(Scratch::take(&SPARE, 3, 2u32).len(), 3);
        assert_eq!(Scratch::take(&SPARE, 20, 2u32).len(), 20);
    }
}
