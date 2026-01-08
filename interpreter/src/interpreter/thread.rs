use std::{cell::UnsafeCell, collections::HashMap, path::PathBuf, sync::Arc};

use slotmap::new_key_type;

use crate::{
    interpreter::{Depend, element::ElementId, file::FileId, module::ModuleId}, new_type, utils::{async_lockfree_stack::Stack, moss}
};

pub struct Thread {
    pub r#mut: UnsafeCell<ThreadMut>,
    pub remote: ThreadRemote,
}

impl Thread {
    pub fn new(module_ids: Vec<ModuleId>) -> Self {
        Self {
            r#mut: UnsafeCell::new(ThreadMut {
                modules: module_ids,
                add_module_delay: AddModuleDelay {
                    files: Default::default(),
                    scopes: Default::default(),
                },
            }),
            remote: ThreadRemote {
                channel: Arc::new(Stack::new()),
            },
        }
    }
}

pub struct ThreadMut {
    pub modules: Vec<ModuleId>,
    pub add_module_delay: AddModuleDelay,
}

pub struct ThreadRemote {
    pub channel: Arc<Stack<Signal>>,
}

pub enum Signal {
    Depend(Depend),
    Resolve(ElementId),
}

new_type! {
    #[derive(Clone,Copy,PartialEq,Debug)]
    pub ThreadId = usize
}

pub struct AddModuleDelay {
    pub files: HashMap<PathBuf, Vec<ElementId>>,
    pub scopes: Vec<AddModuleDelayScope>,
}

pub struct AddModuleDelayScope {
    pub file: FileId,
    pub scope: moss::Scope<'static>,
    pub element: ElementId,
}
