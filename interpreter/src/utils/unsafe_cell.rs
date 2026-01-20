use std::fmt::Debug;

pub struct UnsafeCell<T>(std::cell::UnsafeCell<T>);

unsafe impl<T> Sync for UnsafeCell<T> {}

impl<T> UnsafeCell<T> {
    pub fn new(value: T) -> Self {
        Self(std::cell::UnsafeCell::new(value))
    }
    pub unsafe fn as_ref_unchecked(&self) -> &T {
        unsafe { self.0.as_ref_unchecked() }
    }
    pub unsafe fn as_mut_unchecked(&self) -> &mut T {
        unsafe { self.0.as_mut_unchecked() }
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }
}

impl<T: Debug> Debug for UnsafeCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.as_ref_unchecked().fmt(f) }
    }
}
