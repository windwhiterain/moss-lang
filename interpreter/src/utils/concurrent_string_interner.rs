use std::{
    hash::Hash,
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

use dashmap::DashMap;
use hashbrown::{DefaultHashBuilder, HashMap, hash_map::RawEntryMut};
use parking_lot::RwLock;
use sharded_slab::Slab;
use std::hash::BuildHasher;
use std::hash::Hasher;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StringId(usize);

pub struct SymbolAllocator {
    next: AtomicUsize,
}
impl SymbolAllocator {
    pub fn new() -> Self {
        Self {
            next: AtomicUsize::new(0),
        }
    }
    pub fn alloc(&self) -> StringId {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        StringId(id)
    }
}

pub struct Interner {
    map: HashMap<usize, ()>,
    id2strings: Vec<String>,
    hash_builder: DefaultHashBuilder,
}

impl Interner {
    pub fn new() -> Self {
        let hash_builder = DefaultHashBuilder::default();
        Self {
            map: HashMap::with_hasher(hash_builder.clone()),
            id2strings: Default::default(),
            hash_builder,
        }
    }
    pub fn get_or_intern(&mut self, s: &str) -> StringId {
        let mut hasher = self.hash_builder.build_hasher();
        s.hash(&mut hasher);
        let hash = hasher.finish();
        let entry = self.map.raw_entry_mut().from_hash(hash, |id| {
            // SAFETY: This is safe because we only operate on symbols that
            //         we receive from our backend making them valid.
            s == self.id2strings[*id]
        });
        let (&mut id, &mut ()) = match entry {
            RawEntryMut::Occupied(occupied) => occupied.into_key_value(),
            RawEntryMut::Vacant(vacant) => {
                let id = self.id2strings.len();
                self.id2strings.push(s.to_string());
                vacant.insert_with_hasher(hash, id, (), |id| {
                    let mut hasher = self.hash_builder.build_hasher();
                    (*self.id2strings[*id]).hash(&mut hasher);
                    hasher.finish()
                })
            }
        };
        StringId(id)
    }
    pub fn resolve(&self, id: StringId) -> impl Deref<Target = str> {
        self.id2strings[id.0].as_str()
    }
    pub fn sync_from(&mut self, concurent: &ConcurentInterner) {
        for i in self.id2strings.len()..concurent.alloc.next.load(Ordering::Relaxed) {
            self.get_or_intern(&concurent.resolve(StringId(i)));
        }
    }
}

pub struct ConcurentInterner {
    map: DashMap<String, StringId>,
    strings: Slab<String>,
    id2string: RwLock<Vec<usize>>,
    alloc: SymbolAllocator,
}

impl ConcurentInterner {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
            strings: Default::default(),
            id2string: RwLock::new(Default::default()),
            alloc: SymbolAllocator::new(),
        }
    }

    pub fn get_or_intern(&self, s: &str) -> StringId {
        if let Some(sym) = self.map.get(s) {
            return *sym;
        }

        let entry = self.map.entry(s.to_string()).or_insert_with(|| {
            let sym = self.alloc.alloc();
            let mut vec = self.id2string.write();
            vec.push(self.strings.insert(s.to_string()).unwrap());
            sym
        });
        *entry
    }

    pub fn resolve(&self, id: StringId) -> impl Deref<Target = str> {
        AsStr(self.strings.get(self.id2string.read()[id.0]).unwrap())
    }

    pub fn sync_from(&mut self, interner: &Interner) {
        for i in self.alloc.next.load(Ordering::Relaxed)..interner.id2strings.len() {
            self.get_or_intern(&interner.resolve(StringId(i)));
        }
    }
}

pub struct AsStr<T: Deref<Target = String>>(T);

impl<T: Deref<Target = String>> Deref for AsStr<T> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}
