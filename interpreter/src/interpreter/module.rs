use crate::{gen_pools, interpreter::{Id, function::Function}, utils::pool::Pool};
use slotmap::new_key_type;
use std::{cell::UnsafeCell, fmt::Debug, sync::OnceLock};

use crate::interpreter::{
    element::Element,
    scope::{Scope, ScopeAuthored},
};

gen_pools! {
    #[derive(Debug)]
    pub Pools{scopes:Scope,elements:Element,functions:Function}
}

#[derive(Debug)]
pub struct ModuleLocal {
    pub pools: Pools,
    pub authored: Option<ScopeAuthored>,
    pub dependants: Vec<Id<Element>>,
    pub root_scope: Option<Id<Scope>>,
    pub unresolved_count: usize,
}

pub struct Module {
    pub local: UnsafeCell<ModuleLocal>,
    pub root_scope: OnceLock<Id<Scope>>,
}

impl Debug for Module{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module").field("local", unsafe { self.local.as_ref_unchecked() }).field("root_scope", &self.root_scope).finish()
    }
}

impl ModuleLocal {
    pub fn has_runed(&self) -> bool {
        self.root_scope.is_some()
    }
    pub fn is_resolved(&self) -> bool {
        self.unresolved_count == 0
    }
}

impl Module {
    pub fn new(authored: Option<ScopeAuthored>, resolved: bool) -> Self {
        Self {
            local: UnsafeCell::new(ModuleLocal {
                pools: Default::default(),
                authored,
                dependants: Default::default(),
                root_scope: Default::default(),
                unresolved_count: if resolved { 0 } else { 1 },
            }),
            root_scope: Default::default(),
        }
    }
}

new_key_type! {pub struct ModuleId;}
