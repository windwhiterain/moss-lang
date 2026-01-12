use std::mem::transmute;

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
pub mod spmr_vec;
pub mod type_key;
