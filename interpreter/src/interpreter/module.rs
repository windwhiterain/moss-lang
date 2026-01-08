use crate::interpreter::{element::RemoteInModuleElementId, scope::RemoteInModuleScopeId};
use slotmap::{SlotMap, new_key_type};
use std::{
    cell::{OnceCell, UnsafeCell},
    sync::OnceLock,
};

use crate::interpreter::{
    element::{Element, ElementId, ElementRemote, InModuleElementId},
    scope::{LocalInModuleScopeId, Scope, ScopeAuthored, ScopeRemote},
};

use crate::utils::type_key::SimrVec as KeySimrVec;

pub struct ModuleRemote {
    pub scopes: KeySimrVec<RemoteInModuleScopeId, ScopeRemote>,
    pub elements: KeySimrVec<RemoteInModuleElementId, ElementRemote>,
    pub root_scope: OnceLock<RemoteInModuleScopeId>,
}

pub struct ModuleCell {
    pub scopes: SlotMap<LocalInModuleScopeId, Scope>,
    pub elements: SlotMap<InModuleElementId, Element>,
    pub authored: Option<ScopeAuthored>,
    pub dependants: Vec<ElementId>,
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
