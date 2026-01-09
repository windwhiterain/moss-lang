use slotmap::{Key, SecondaryMap};

use crate::utils::erase;

#[derive(Default)]
pub struct Link<K> {
    prev: Option<K>,
    next: Option<K>,
}
#[derive(Default)]
pub struct List<K: Key> {
    secondary: SecondaryMap<K, Link<K>>,
    first: Option<K>,
}

impl<K: Copy + Key> List<K> {
    pub fn push(&mut self, key: K) {
        self.secondary.insert(key, Default::default());
        if let Some(first) = self.first {
            let link = &mut self.secondary[first];
            link.prev = Some(key);
            let new_link = &mut self.secondary[key];
            new_link.next = Some(first);
        }
        self.first = Some(key.clone());
    }
    pub fn remove(&mut self, key: K) {
        let link = &erase(self).secondary[key];
        if let Some(prev) = link.prev {
            self.secondary[prev].next = link.next;
        }
        if let Some(next) = link.next {
            self.secondary[next].prev = link.prev;
        }
        if self.first.unwrap() == key {
            self.first = link.next;
        }
        self.secondary.remove(key);
    }
    pub fn len(&self) -> usize {
        self.secondary.len()
    }
    pub fn iter(&self) -> ListIterator<'_, K> {
        ListIterator {
            list: self,
            next: None,
        }
    }
    pub fn retain(&mut self, if_remove: impl Fn(K) -> bool) {
        let mut key_iter = self.first;
        loop {
            let Some(key) = key_iter else {
                break;
            };
            let link = &self.secondary[key];
            key_iter = link.next;
            if if_remove(key) {
                self.remove(key);
            }
        }
    }
    pub fn clear(&mut self) {
        self.first = None;
        self.secondary.clear();
    }
}

pub struct ListIterator<'a, K: Key + Copy> {
    pub list: &'a List<K>,
    pub next: Option<K>,
}

impl<'a, K: Key + Copy> Iterator for ListIterator<'a, K> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = if let Some(next) = self.next {
            Some(next)
        } else {
            self.list.first
        };
        if let Some(ret) = ret {
            self.next = self.list.secondary[ret].next;
        }
        ret
    }
}
