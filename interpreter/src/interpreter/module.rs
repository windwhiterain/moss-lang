use crate::interpreter::{element::RemoteInModuleElementId, scope::RemoteInModuleScopeId};
use slotmap::new_key_type;
use std::{
    cell::{OnceCell, UnsafeCell},
    sync::OnceLock,
};

use crate::interpreter::{
    element::{Element, ElementId, ElementRemote, InModuleElementId},
    scope::{InModuleScopeId, Scope, ScopeAuthored, ScopeRemote},
};

use crate::utils::type_key::{SimrVec as KeySimrVec, Vec as KeyVec};

pub struct ModuleRemote {
    pub scopes: KeySimrVec<RemoteInModuleScopeId, ScopeRemote>,
    pub elements: KeySimrVec<RemoteInModuleElementId, ElementRemote>,
    pub root_scope: OnceLock<RemoteInModuleScopeId>,
}

pub struct Module {
    pub scopes: KeyVec<InModuleScopeId, Scope>,
    pub elements: KeyVec<InModuleElementId, Element>,
    pub authored: Option<ScopeAuthored>,
    pub dependants: Vec<ElementId>,
    pub root_scope: OnceCell<InModuleScopeId>,
    pub unresolved_count: usize,
}

impl Module {
    pub fn has_parsed(&self) -> bool {
        self.root_scope.get().is_some()
    }
    pub fn is_resolved(&self) -> bool {
        self.unresolved_count == 0
    }
}

pub struct ConcurrentModule {
    /// # Safety
    /// 
    /// only access in one thread
    pub local: UnsafeCell<Module>,
    pub remote: ModuleRemote,
}

impl ConcurrentModule {
    pub fn new(source: ScopeAuthored) -> Self {
        Self {
            local: UnsafeCell::new(Module {
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
