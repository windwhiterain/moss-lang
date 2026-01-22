use std::{
    cell::UnsafeCell,
    mem::{MaybeUninit, transmute},
};

pub fn erase_mut<'a, 'b, T: ?Sized>(x: &'a mut T) -> &'b mut T {
    unsafe { transmute(x) }
}

pub fn erase<'a, 'b, T: ?Sized>(x: &'a T) -> &'b T {
    unsafe { transmute(x) }
}

#[macro_export]
macro_rules! erase_struct {
    ($x:expr) => {{ unsafe { std::mem::transmute($x) } }};
}

pub use crate::type_sitter_lang::moss;

pub mod async_lockfree_stack;
pub mod concurrent_string_interner;
pub mod secondary_linked_list;
pub mod pool;
pub mod spmr_vec;
pub mod type_key;
pub mod unsafe_cell;
pub mod contexted;

pub fn new_uninit_cell_slice<T>(capacity: usize) -> Box<[UnsafeCell<MaybeUninit<T>>]> {
    let mut boxed = Box::<[MaybeUninit<T>]>::new_uninit_slice(capacity);

    let ptr = boxed.as_mut_ptr() as *mut UnsafeCell<MaybeUninit<T>>;
    let len = boxed.len();

    std::mem::forget(boxed);

    unsafe { Box::from_raw(std::slice::from_raw_parts_mut(ptr, len)) }
}
