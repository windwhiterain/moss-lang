use arc_swap::ArcSwap;
use sharded_slab::Clear;

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
    buf: UnsafeCell<Box<[MaybeUninit<T>]>>,
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
            buf: UnsafeCell::new(Box::new_uninit_slice(capacity)),
            next_idx: AtomicUsize::new(0),
        }
    }
    pub fn capacity(&self) -> usize {
        unsafe { self.buf.as_ref_unchecked().len() }
    }
    pub fn len(&self) -> usize {
        self.next_idx.load(Ordering::Relaxed)
    }
    pub unsafe fn push(&self, value: T) -> Result<usize, T> {
        let next_idx = self.next_idx.load(Ordering::Relaxed);
        let buf = unsafe { self.buf.as_mut_unchecked() };
        if next_idx >= buf.len() {
            return Err(value);
        }
        unsafe {
            buf[next_idx].as_mut_ptr().write(value);
        }
        self.next_idx.store(next_idx + 1, Ordering::Release);
        Ok(next_idx)
    }
    pub fn get(&self, idx: usize) -> Option<&T> {
        let next_idx = self.next_idx.load(Ordering::Acquire);
        if idx >= next_idx {
            return None;
        }
        let buf = unsafe { self.buf.as_ref_unchecked() };
        unsafe { Some(buf[idx].assume_init_ref()) }
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let next_idx = self.next_idx.load(Ordering::Acquire);
        let buf = unsafe { self.buf.as_ref_unchecked() };
        unsafe { (0..next_idx).map(|idx| &*buf[idx].as_ptr()) }
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        let next_idx = *self.next_idx.get_mut();
        let buf = unsafe { self.buf.as_mut_unchecked() };
        unsafe { (0..next_idx).map(|idx| &mut *buf[idx].as_mut_ptr()) }
    }
    pub unsafe fn copy_from(&mut self, other: &Self) {
        unsafe {
            copy_nonoverlapping(
                other.buf.as_ref_unchecked().as_ptr(),
                self.buf.get_mut().as_mut_ptr(),
                other.len(),
            )
        }
        self.next_idx.store(other.len(), Ordering::Release);
    }
    pub fn push_single_thread(&mut self, value: T) -> Result<usize, T> {
        let next_idx = self.next_idx.get_mut();
        if let Some(x) = self.buf.get_mut().get_mut(*next_idx) {
            let idx = *next_idx;
            x.write(value);
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
        let buf = unsafe { self.buf.as_mut_unchecked() };
        unsafe { Some(buf[idx].assume_init_mut()) }
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
    guard: arc_swap::Guard<Arc<Array<T>>>,
    value: &'a T,
}

impl<'a, T: 'a> Deref for Guard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct IterGuard<'a, T: 'a, Iter: 'a + Iterator<Item = &'a T>> {
    guard: arc_swap::Guard<Arc<Array<T>>>,
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
    pub unsafe fn push(&self, value: T) -> usize {
        let array = self.array.load();
        match unsafe { array.push(value) } {
            Ok(idx) => idx,
            Err(value) => {
                let mut new_array = Array::<T>::new(max(array.capacity() * 2, INITIAL_CAPACITY));
                unsafe { new_array.copy_from(&array) };
                let idx = new_array
                    .push_single_thread(value)
                    .unwrap_or_else(|_| unreachable!());
                self.array.store(Arc::new(new_array));
                idx
            }
        }
    }
    pub fn get(&self, idx: usize) -> Option<Guard<'_, T>> {
        let guard = self.array.load();
        let value = erase(guard.get(idx)?);
        Some(Guard { guard, value })
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let guard = self.array.load();
        let value = erase(&guard).iter();
        IterGuard {
            guard,
            value,
            _p: PhantomData,
        }
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.array.get_mut().unwrap().iter_mut()
    }
    pub fn push_single_thread(&mut self, value: T) -> usize {
        let array = self.array.get_mut().unwrap();
        match array.push_single_thread(value) {
            Ok(idx) => idx,
            Err(value) => {
                let mut new_array = Array::<T>::new(max(array.capacity() * 2, INITIAL_CAPACITY));
                unsafe { new_array.copy_from(&array) };
                let idx = new_array
                    .push_single_thread(value)
                    .unwrap_or_else(|_| unreachable!());
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
        println!("{:?}", vec.push(1));
        println!("{:?}", vec.len());
        println!("{:?}", vec.push(2));
        println!("{:?}", vec.len());
        println!("{:?},{:?}", &*vec.get(0).unwrap(), &*vec.get(1).unwrap());
    }
}
