use std::{fmt::Debug, mem::MaybeUninit, sync::atomic::AtomicPtr};

pub struct Pool<T> {
    chuncks: Vec<Box<[MaybeUninit<T>]>>,
    current_chunck_idx: usize,
    next_idx: usize,
    length: usize,
}

impl<T: Debug> Debug for Pool<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self {
            chuncks: Default::default(),
            current_chunck_idx: 0,
            next_idx: Default::default(),
            length: Default::default(),
        }
    }
}

impl<T> Pool<T> {
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            return Self::default();
        }
        Self {
            chuncks: vec![Box::new_uninit_slice(capacity)],
            current_chunck_idx: 0,
            next_idx: 0,
            length: 0,
        }
    }
    pub fn insert(&mut self, value: T) -> *const T {
        if self.chuncks.is_empty() {
            self.chuncks.push(Box::new_uninit_slice(8));
        }
        let current_length = self.chuncks.get(self.current_chunck_idx).unwrap().len();
        if self.next_idx == current_length {
            self.current_chunck_idx += 1;
            if self.current_chunck_idx == self.chuncks.len() {
                self.chuncks.push(Box::new_uninit_slice(current_length * 2));
            }
            self.next_idx = 0;
        }
        let current_chunck = self.chuncks.get_mut(self.current_chunck_idx).unwrap();
        let raw_item = current_chunck.get_mut(self.next_idx).unwrap();
        let ret = raw_item.write(value) as *const T;
        self.next_idx += 1;
        self.length += 1;
        ret
    }
    pub fn clear(&mut self) {
        for raw_item in self.iter_raw_mut() {
            unsafe { raw_item.assume_init_drop() };
        }
        self.current_chunck_idx = 0;
        self.next_idx = 0;
        self.length = 0;
    }
    pub fn iter_raw(&self) -> impl Iterator<Item = &MaybeUninit<T>> {
        self.chuncks.iter().flat_map(|x| x.iter()).take(self.len())
    }
    pub fn iter_raw_mut(&mut self) -> impl Iterator<Item = &mut MaybeUninit<T>> {
        let length = self.length;
        self.chuncks
            .iter_mut()
            .flat_map(|x| x.iter_mut())
            .take(length)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.iter_raw().map(|x| unsafe { x.assume_init_ref() })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.iter_raw_mut().map(|x| unsafe { x.assume_init_mut() })
    }
    pub fn len(&self) -> usize {
        self.length
    }
}

impl<T> Drop for Pool<T> {
    fn drop(&mut self) {
        for raw_item in self.iter_raw_mut() {
            unsafe { raw_item.assume_init_drop() };
        }
    }
}

#[test]
fn test() {
    use core::cell::Cell;
    thread_local! {
        static DROP_COUNT: Cell<usize> = Cell::new(0);
    }
    struct DropCounter(usize);
    impl Debug for DropCounter {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
    impl DropCounter {
        fn new(value: usize) -> Self {
            DROP_COUNT.with(|x| x.set(x.get() + 1));
            Self(value)
        }
    }
    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT.with(|x| x.set(x.get() - 1));
        }
    }
    {
        let mut pool = Pool::<DropCounter>::new(1);
        for i in 0..16 {
            println!("{:?}", pool.len());
            println!("{:?}", pool);
            println!("{:?}", pool.insert(DropCounter::new(i)));
        }
        pool.clear();
        assert!(DROP_COUNT.get() == 0);
        for i in 0..16 {
            println!("{:?}", pool.len());
            println!("{:?}", pool);
            println!("{:?}", pool.insert(DropCounter::new(i)));
        }
    }
    assert!(DROP_COUNT.get() == 0);
}
