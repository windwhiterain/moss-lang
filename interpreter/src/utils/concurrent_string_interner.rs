use hashbrown::{DefaultHashBuilder, HashMap, hash_map::RawEntryMut};
use std::hash::BuildHasher;
use std::hash::Hasher;
use std::{hash::Hash, ops::Deref};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StringId(usize);

impl StringId {
    pub const DUMMY: StringId = StringId(0);
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
        for i in self.id2strings.len()..concurent.inner.len() {
            self.get_or_intern(&concurent.resolve(StringId(i)));
        }
    }
}

impl inturn::InternerSymbol for StringId {
    fn try_from_usize(id: usize) -> Option<Self> {
        Some(Self(id))
    }

    fn to_usize(self) -> usize {
        self.0
    }
}

pub struct ConcurentInterner {
    inner: inturn::Interner<StringId>,
}

impl ConcurentInterner {
    pub fn new() -> Self {
        Self {
            inner: inturn::Interner::<StringId>::with_capacity_and_hasher(0, Default::default()),
        }
    }

    pub fn get_or_intern(&self, s: &str) -> StringId {
        self.inner.intern(s)
    }

    pub fn resolve(&self, id: StringId) -> impl Deref<Target = str> {
        self.inner.resolve(id)
    }

    pub fn sync_from(&mut self, interner: &Interner) {
        for i in self.inner.len()..interner.id2strings.len() {
            self.get_or_intern(&interner.resolve(StringId(i)));
        }
    }
}
