use std::{cell::UnsafeCell, collections::HashMap, path::PathBuf, sync::Arc};

use type_sitter::UntypedNode;

use crate::{
    interpreter::{element::ElementId, file::FileId, module::ModuleId},
    new_type,
    utils::{async_lockfree_stack::Stack, moss},
};

new_type! {
    #[derive(Clone,Copy,PartialEq,Debug)]
    pub ThreadId = usize
}

pub struct ThreadLocal {
    pub modules: Vec<ModuleId>,
    pub add_module_delay: AddModuleDelay,
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
                add_module_delay: AddModuleDelay {
                    files: Default::default(),
                },
            }),
            remote: ThreadRemote {
                channel: Arc::new(Stack::new()),
            },
        }
    }
}

pub struct Depend {
    pub dependant: ElementId,
    pub dependency: ElementId,
    pub node: UntypedNode<'static>,
}

pub enum Signal {
    Depend(Depend),
    Resolve(ElementId),
}

pub struct AddModuleDelay {
    pub files: HashMap<PathBuf, Vec<ElementId>>,
}

pub struct AddModuleDelayScope {
    pub file: FileId,
    pub scope: moss::Scope<'static>,
    pub element: ElementId,
}
