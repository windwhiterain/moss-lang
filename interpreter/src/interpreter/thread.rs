use std::{cell::UnsafeCell, collections::HashMap, path::PathBuf, sync::Arc};

use type_sitter::UntypedNode;

use crate::{
    interpreter::{Id, element::Element, file::FileId, module::ModuleId},
    new_type,
    utils::{async_lockfree_stack::Stack, moss},
};

new_type! {
    #[derive(Clone,Copy,PartialEq,Debug)]
    pub ThreadId = usize
}

pub struct ThreadLocal {
    pub modules: Vec<ModuleId>,
}

pub struct ThreadRemote {
    pub channel: Arc<Stack<Signal>>,
}

pub struct Thread {
    /// # Safety
    ///
    /// only access in one thread
    pub local: UnsafeCell<ThreadLocal>,
    pub remote: ThreadRemote,
}

unsafe impl Sync for Thread {}

impl Thread {
    pub fn new(module_ids: Vec<ModuleId>) -> Self {
        Self {
            local: UnsafeCell::new(ThreadLocal {
                modules: module_ids,
            }),
            remote: ThreadRemote {
                channel: Arc::new(Stack::new()),
            },
        }
    }
}

pub struct Depend {
    pub dependant: Id<Element>,
    pub dependency: Id<Element>,
    pub source: Option<UntypedNode<'static>>,
}

pub enum Signal {
    Depend(Depend),
    Resolve(Id<Element>),
}
