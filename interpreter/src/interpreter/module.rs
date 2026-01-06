use std::{
    cell::{OnceCell, UnsafeCell},
    path::PathBuf,
    sync::OnceLock,
};

use sharded_slab::Slab;
use slotmap::{SlotMap, new_key_type};

use crate::{
    interpreter::{
        element::{Element, ElementRemote, LocalElementId, LocalInModuleElementId},
        file::FileId,
        scope::{LocalInModuleScopeId, Scope, ScopeAuthored, ScopeRemote},
    },
    utils::moss,
};

pub struct ModuleRemote {
    pub scopes: Slab<ScopeRemote>,
    pub elements: Slab<ElementRemote>,
    pub root_scope: OnceLock<usize>,
}

pub struct ModuleCell {
    pub scopes: SlotMap<LocalInModuleScopeId, Scope>,
    pub elements: SlotMap<LocalInModuleElementId, Element>,
    pub authored: Option<ScopeAuthored>,
    pub dependants: Vec<LocalElementId>,
    pub root_scope: OnceCell<LocalInModuleScopeId>,
    pub unresolved_count: usize,
}

impl ModuleCell {
    pub fn has_parsed(&self) -> bool {
        self.root_scope.get().is_some()
    }
    pub fn is_resolved(&self) -> bool {
        self.unresolved_count == 0
    }
}

pub struct Module {
    pub cell: UnsafeCell<ModuleCell>,
    pub remote: ModuleRemote,
}

impl Module {
    pub fn new(source: ScopeAuthored) -> Self {
        Self {
            cell: UnsafeCell::new(ModuleCell {
                scopes: Default::default(),
                elements: Default::default(),
                authored: Some(source),
                dependants: Default::default(),
                root_scope: Default::default(),
                unresolved_count: 1,
            }),
            remote: ModuleRemote {
                scopes: Default::default(),
                elements: Default::default(),
                root_scope: Default::default(),
            },
        }
    }
}

new_key_type! {pub struct ModuleId;}

pub enum ModuleAuthored {
    File {
        path: PathBuf,
    },
    Scope {
        file: FileId,
        source: moss::Scope<'static>,
    },
}
