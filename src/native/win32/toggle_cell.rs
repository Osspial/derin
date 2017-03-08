use std::cell::{Cell, UnsafeCell};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MutState {
    Extern,
    Intern,
    InternBorrowed
}

/// A cell that can toggle between "external" and "internal" borrowing modes.
///
/// In external mode, borrows are checked by the Rust borrow checker at compile time and performed
/// with the `get` and `get_mut` methods. In internal mode, borrows are checked at run-time and
/// performed with the `borrow_mut` method. Performing an external borrow automatically switches to
/// internal mode, but internal mode must be explicitly activated with the `intern_mode` method.
#[derive(Default)]
pub struct ToggleCell<T> {
    data: UnsafeCell<T>,
    mut_state: Cell<MutState>
}

impl<T> ToggleCell<T> {
    /// Create a new `ToggleCell`.
    pub fn new(data: T) -> ToggleCell<T> {
        ToggleCell {
            data: UnsafeCell::new(data),
            mut_state: Cell::new(MutState::Extern)
        }
    }

    /// Switch from externally borrowed mode to internally borrowed mode. This requirse `&mut self`
    /// because that guarantees that no borrows are being performed on the cell.
    pub fn intern_mode(&mut self) {
        self.mut_state.set(MutState::Intern);
    }

    /// Get a reference to the cell's data through external borrowing.
    ///
    /// # Panics
    /// This function will panic if an internal borrow is currently being performed by `borrow_mut`.
    pub fn get(&self) -> &T {
        if MutState::InternBorrowed != self.mut_state.get() {
            self.mut_state.set(MutState::Extern);
            unsafe{ &*self.data.get() }

        } else {
            panic!("ToggleCell externally borrowed while internally borrowed");
        }
    }

    /// Get a mutable reference to the cell's data through external borrowing.
    pub fn get_mut(&mut self) -> &mut T {
        self.mut_state.set(MutState::Extern);

        unsafe{ &mut *self.data.get() }
    }

    /// Get a mutable reference to the cell's data through internal borrowing.
    ///
    /// # Panics
    /// This function will panic if the cell isn't in internal borrowing mode, or if an internal
    /// borrow is already being performed.
    pub fn borrow_mut(&self) -> RefMut<T> {
        if MutState::Intern == self.mut_state.get() {
            self.mut_state.set(MutState::InternBorrowed);
            RefMut {
                t_cell: self
            }
        } else {
            panic!("ToggleCell in invalid state to be borrowed: {:?}", self.mut_state.get());
        }
    }

    pub fn into_inner(self) -> T {
        unsafe{ self.data.into_inner() }
    }
}

pub struct RefMut<'a, T: 'a> {
    t_cell: &'a ToggleCell<T>
}

impl<'a, T: 'a> Deref for RefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe{ &*self.t_cell.data.get() }
    }
}

impl<'a, T: 'a> DerefMut for RefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe{ &mut *self.t_cell.data.get() }
    }
}

impl<'a, T: 'a> Drop for RefMut<'a, T> {
    fn drop(&mut self) {
        self.t_cell.mut_state.set(MutState::Intern);
    }
}

impl Default for MutState {
    fn default() -> MutState {
        MutState::Extern
    }
}
