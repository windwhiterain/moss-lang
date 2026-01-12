//! Single producer, multi readers.
use arc_swap::ArcSwap;

use std::{
    cell::UnsafeCell,
    cmp::max,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::Deref,
    ptr::copy_nonoverlapping,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::utils::erase;

static INITIAL_CAPACITY: usize = 8;

#[derive(Debug)]
pub struct Array<T> {
    /// # Safety
    ///
    /// readers can only read `idx < next_idx`, one writer can only write `dix == next_idx`
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    next_idx: AtomicUsize,
}

unsafe impl<T> Sync for Array<T> {}

impl<T> Default for Array<T> {
    fn default() -> Self {
        Self {
            buf: Default::default(),
            next_idx: Default::default(),
        }
    }
}

impl<T> Array<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: {
                let mut boxed = Box::<[MaybeUninit<T>]>::new_uninit_slice(capacity);

                let ptr = boxed.as_mut_ptr() as *mut UnsafeCell<MaybeUninit<T>>;
                let len = boxed.len();

                std::mem::forget(boxed);

                unsafe { Box::from_raw(std::slice::from_raw_parts_mut(ptr, len)) }
            },
            next_idx: AtomicUsize::new(0),
        }
    }
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }
    pub fn len(&self) -> usize {
        self.next_idx.load(Ordering::Relaxed)
    }
    /// # Safety
    ///
    /// can only called by the only writer
    pub unsafe fn push_concurrent(&self, value: T) -> Result<usize, T> {
        let next_idx = self.next_idx.load(Ordering::Relaxed);
        if next_idx >= self.buf.len() {
            return Err(value);
        }
        unsafe {
            self.buf[next_idx]
                .as_mut_unchecked()
                .as_mut_ptr()
                .write(value);
        }
        self.next_idx.store(next_idx + 1, Ordering::Release);
        Ok(next_idx)
    }
    pub fn get_concurrent(&self, idx: usize) -> Option<&T> {
        let next_idx = self.next_idx.load(Ordering::Acquire);
        if idx >= next_idx {
            return None;
        }
        unsafe { Some(self.buf[idx].as_ref_unchecked().assume_init_ref()) }
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let next_idx = self.next_idx.load(Ordering::Acquire);
        unsafe { (0..next_idx).map(|idx| self.buf[idx].as_ref_unchecked().assume_init_ref()) }
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        let next_idx = *self.next_idx.get_mut();
        unsafe { (0..next_idx).map(|idx| self.buf[idx].as_mut_unchecked().assume_init_mut()) }
    }
    /// # Safety
    ///
    /// can only called by the only writer
    pub unsafe fn copy_from(&mut self, other: &Self) {
        unsafe { copy_nonoverlapping(other.buf.as_ptr(), self.buf.as_mut_ptr(), other.len()) }
        self.next_idx.store(other.len(), Ordering::Release);
    }
    pub fn push(&mut self, value: T) -> Result<usize, T> {
        let next_idx = self.next_idx.get_mut();
        if let Some(x) = self.buf.get_mut(*next_idx) {
            let idx = *next_idx;
            x.get_mut().write(value);
            *next_idx += 1;
            Ok(idx)
        } else {
            Err(value)
        }
    }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        let next_idx = *self.next_idx.get_mut();
        if idx >= next_idx {
            return None;
        }
        unsafe { Some(self.buf[idx].get_mut().assume_init_mut()) }
    }
    pub fn clear(&mut self) {
        *self.next_idx.get_mut() = 0;
    }
}

#[derive(Debug)]
pub struct Vec<T> {
    array: ArcSwap<Array<T>>,
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self {
            array: Default::default(),
        }
    }
}

pub struct Guard<'a, T> {
    _guard: arc_swap::Guard<Arc<Array<T>>>,
    value: &'a T,
}

impl<'a, T: 'a> Deref for Guard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct IterGuard<'a, T: 'a, Iter: 'a + Iterator<Item = &'a T>> {
    _guard: arc_swap::Guard<Arc<Array<T>>>,
    value: Iter,
    _p: PhantomData<&'a ()>,
}

impl<'a, T: 'a, Iter: 'a + Iterator<Item = &'a T>> Iterator for IterGuard<'a, T, Iter> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.next()
    }
}

impl<T> Vec<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            array: ArcSwap::from_pointee(Array::new(capacity)),
        }
    }
    pub fn len(&self) -> usize {
        self.array.load().len()
    }
    /// # Safety
    ///
    /// can only called by the only writer
    pub unsafe fn push_concurrent(&self, value: T) -> usize {
        let array = self.array.load();
        match unsafe { array.push_concurrent(value) } {
            Ok(idx) => idx,
            Err(value) => {
                let mut new_array = Array::<T>::new(max(array.capacity() * 2, INITIAL_CAPACITY));
                unsafe { new_array.copy_from(&array) };
                let idx = new_array.push(value).unwrap_or_else(|_| unreachable!());
                self.array.store(Arc::new(new_array));
                idx
            }
        }
    }
    pub fn get_concurrent(&self, idx: usize) -> Option<Guard<'_, T>> {
        let guard = self.array.load();
        let value = erase(guard.get_concurrent(idx)?);
        Some(Guard {
            _guard: guard,
            value,
        })
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let guard = self.array.load();
        let value = erase(&guard).iter();
        IterGuard {
            _guard: guard,
            value,
            _p: PhantomData,
        }
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.array.get_mut().unwrap().iter_mut()
    }
    pub fn push(&mut self, value: T) -> usize {
        let array = self.array.get_mut().unwrap();
        match array.push(value) {
            Ok(idx) => idx,
            Err(value) => {
                let mut new_array = Array::<T>::new(max(array.capacity() * 2, INITIAL_CAPACITY));
                unsafe { new_array.copy_from(&array) };
                let idx = new_array.push(value).unwrap_or_else(|_| unreachable!());
                *self.array.get_mut().unwrap() = new_array;
                idx
            }
        }
    }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.array.get_mut().unwrap().get_mut(idx)
    }
    pub fn clear(&mut self) {
        self.array.get_mut().unwrap().clear();
    }
}

#[test]
fn test() {
    unsafe {
        let mut vec = Vec::<usize>::default();
        println!("{:?}", vec.len());
        println!("{:?}", vec.push_concurrent(1));
        println!("{:?}", vec.len());
        println!("{:?}", vec.push_concurrent(2));
        println!("{:?}", vec.len());
        for i in vec.iter() {
            println!("-{:?}", i);
        }
    }
}
