use std::ops::{Deref, DerefMut};

/// A "move callback vector", which calls callback `C` whenever the memory inside of the vector
/// is moved.
#[derive(Default)]
pub struct MCVec<T, C>
        where for<'a> C: Fn(&mut T)
{
    vec: Vec<T>,
    callback: C
}

impl<T, C> MCVec<T, C>
        where for<'a> C: Fn(&mut T)
{
    pub fn new(callback: C) -> MCVec<T, C> {
        MCVec::with_capacity(callback, 0)
    }

    pub fn with_capacity(callback: C, capacity: usize) -> MCVec<T, C> {
        MCVec {
            vec: Vec::with_capacity(capacity),
            callback: callback
        }
    }

    pub fn insert(&mut self, index: usize, item: T) {
        let orig_ptr = self.start_ptr();
        self.vec.insert(index, item);

        if orig_ptr != self.start_ptr() {
            for i in &mut self.vec[..] {
                (self.callback)(i);
            }
        } else {
            for i in &mut self.vec[index..] {
                (self.callback)(i);
            }
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        let ret = self.vec.remove(index);
        for i in &mut self.vec[index..] {
            (self.callback)(i);
        }
        ret
    }

    /// Replace the item at the given index with the new item, running the callback on the new item.
    /// Note that the callback is NOT run on the item being removed, as the destination memory address
    /// is unknown.
    pub fn replace(&mut self, index: usize, mut item: T) -> T {
        use std::mem;

        assert!(index < self.vec.len());

        let vec_item = unsafe{ self.vec.get_unchecked_mut(index) };
        mem::swap(vec_item, &mut item);
        (self.callback)(vec_item);
        item
    }

    fn start_ptr(&self) -> *const T {
        use std::mem;

        unsafe{ mem::transmute(self.get(0)) }
    }
}

impl<T, C> Deref for MCVec<T, C>
        where for<'a> C: Fn(&mut T)
{
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.vec
    }
}

impl<T, C> DerefMut for MCVec<T, C>
        where for<'a> C: Fn(&mut T)
{
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.vec
    }
}
