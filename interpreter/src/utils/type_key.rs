use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[macro_export]
macro_rules! new_type {
    ( $(#[$outer:meta])* $vis:vis $new:ident = $old:ident ) => {
        $(#[$outer])*
        #[repr(transparent)]
        $vis struct $new($old);

        impl core::convert::From<$old> for $new {
            fn from(value: $old) -> Self {
                $new(value)
            }
        }

        impl core::convert::From<$new> for $old {
            fn from(value: $new) -> Self {
                value.0
            }
        }
    };
}

#[derive(Debug)]
pub struct Vec<K, V>(std::vec::Vec<V>, PhantomData<K>);

impl<K, V> Default for Vec<K, V> {
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}

impl<K: From<usize> + Into<usize>, V> Vec<K, V> {
    pub fn insert(&mut self, value: V) -> K {
        let key = self.0.len();
        self.0.push(value);
        key.into()
    }
    pub fn get(&self, key: K) -> &V {
        self.0.get(key.into()).unwrap()
    }
    pub fn get_mut(&mut self, key: K) -> &mut V {
        self.0.get_mut(key.into()).unwrap()
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn keys(&self) -> impl Iterator<Item = K> {
        (0..self.len()).map(|x| x.into())
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.iter()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.0.iter_mut()
    }
}

#[derive(Debug)]
pub struct SpmrVec<K, V>(crate::utils::spmr_vec::Vec<V>, PhantomData<K>);

impl<K, V> Default for SpmrVec<K, V> {
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}

impl<K: From<usize> + Into<usize>, V> SpmrVec<K, V> {
    /// # Safety
    /// 
    /// can only called by the only writer
    pub unsafe fn insert(&self, value: V) -> K {
        unsafe { self.0.push_concurrent(value).into() }
    }
    pub fn get_concurrent(&self, key: K) -> impl Deref<Target = V> {
        self.0.get_concurrent(key.into()).unwrap()
    }
    pub fn get_mut(&mut self, key: K) -> impl DerefMut<Target = V> {
        self.0.get_mut(key.into()).unwrap()
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn keys(&self) -> impl Iterator<Item = K> {
        (0..self.len()).map(|x| x.into())
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.iter()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.0.iter_mut()
    }
}

#[test]
fn test() {
    unsafe {
        let mut vec = Vec::<usize,usize>::default();
        println!("{:?}", vec.len());
        println!("{:?}", vec.insert(1));
        println!("{:?}", vec.len());
        println!("{:?}", vec.insert(2));
        println!("{:?}", vec.len());
        for i in vec.keys(){
            println!("-{:?}", i);
        }
    }
}
