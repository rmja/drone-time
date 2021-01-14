//! Atomic version of `Option<Box<T>>` inspired by https://github.com/jorendorff/atomicbox

use core::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

/// An atomic `Option<Box<T>>`.
pub struct AtomicOptionBox<T> {
    inner: AtomicPtr<T>,
}

impl<T> AtomicOptionBox<T> {
    /// Create a new `AtomicOptionBox` with a given value.
    pub fn new(option: Option<Box<T>>) -> AtomicOptionBox<T> {
        let ptr = match option {
            Some(data) => Box::into_raw(data),
            None => ptr::null_mut(),
        };
        AtomicOptionBox {
            inner: AtomicPtr::new(ptr),
        }
    }

    /// Create a new `AtomicOptionBox` with the `None` value, useful for static variables.
    pub const fn none() -> AtomicOptionBox<T> {
        AtomicOptionBox {
            inner: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Takes the value out of the option, leaving a None in its place.
    ///
    /// `ordering` must be either `Ordering::AcqRel` or `Ordering::SeqCst`,
    /// as other values would not be safe if `T` contains any data.
    pub fn take(&self, ordering: Ordering) -> Option<Box<T>> {
        self.replace(ptr::null_mut(), ordering)
    }

    /// Swap the current value with `new`, returning the old value.
    ///
    /// `ordering` must be either `Ordering::AcqRel` or `Ordering::SeqCst`,
    /// as other values would not be safe if `T` contains any data.
    pub fn swap(&self, new: Option<Box<T>>, ordering: Ordering) -> Option<Box<T>> {
        let new = new.map_or(ptr::null_mut(), Box::into_raw);
        self.replace(new, ordering)
    }

    /// Store a new value.
    ///
    /// `ordering` must be either `Ordering::AcqRel` or `Ordering::SeqCst`,
    /// as other values would not be safe if `T` contains any data.
    pub fn store(&self, new: Option<Box<T>>, ordering: Ordering) {
        let new = new.map_or(ptr::null_mut(), Box::into_raw);
        self.replace(new, ordering);
    }

    /// Store a new value if and only if the current value is None.
    pub fn try_store(&self, new: Box<T>, ordering: Ordering) -> bool {
        let new = Box::into_raw(new);
        let old = self.inner.compare_and_swap(ptr::null_mut(), new, ordering);
        // Note, it is not safe to read *old if ordering == Ordering::Relaxed.
        // However, we do not read the boxed value as old cannot be a box because
        // of the compare constraint.
        old.is_null()
    }

    fn replace(&self, new: *mut T, ordering: Ordering) -> Option<Box<T>> {
        // It is not safe to read *old if ordering == Ordering::Relaxed
        // as the Box pointer may not yet be available.
        assert!(ordering == Ordering::AcqRel || ordering == Ordering::SeqCst);
        let old = self.inner.swap(new, ordering);
        if old.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(old) })
        }
    }
}

impl<T> Drop for AtomicOptionBox<T> {
    fn drop(&mut self) {
        let ptr = self.inner.load(Ordering::Acquire);
        if !ptr.is_null() {
            unsafe {
                Box::from_raw(ptr);
            }
        }
    }
}
