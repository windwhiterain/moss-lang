use crate::interpreter::{element::RemoteInModuleElementId, scope::RemoteInModuleScopeId};
use slotmap::new_key_type;
use std::{cell::UnsafeCell, sync::OnceLock};

use crate::interpreter::{
    element::{Element, ElementId, ElementRemote, InModuleElementId},
    scope::{InModuleScopeId, Scope, ScopeAuthored, ScopeRemote},
};

use crate::utils::type_key::{SpmrVec as KeySimrVec, Vec as KeyVec};

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
    pub root_scope: Option<InModuleScopeId>,
    pub unresolved_count: usize,
}

impl Module {
    pub fn has_runed(&self) -> bool {
        self.root_scope.is_some()
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

unsafe impl Sync for ConcurrentModule {}

impl ConcurrentModule {
    pub fn new(authored: Option<ScopeAuthored>, resolved: bool) -> Self {
        Self {
            local: UnsafeCell::new(Module {
                scopes: Default::default(),
                elements: Default::default(),
                authored,
                dependants: Default::default(),
                root_scope: Default::default(),
                unresolved_count: if resolved { 0 } else { 1 },
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
