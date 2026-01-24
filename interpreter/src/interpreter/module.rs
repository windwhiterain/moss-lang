use crate::{
    gen_pools,
    interpreter::{
        Id,
        file::FileId,
        function::{Function, FunctionBody, Param},
    },
    utils::pool::Pool,
};
use slotmap::new_key_type;
use std::{cell::UnsafeCell, fmt::Debug};

use crate::interpreter::{
    element::Element,
    scope::{Scope, ScopeAuthored},
};

gen_pools! {
    #[derive(Debug)]
    pub Pools{scopes:Scope,elements:Element,functions:Function,params:Param,function_bodies:FunctionBody}
}

#[derive(Debug)]
pub struct ModuleLocal {
    pub pools: Pools,
    pub authored: Option<ScopeAuthored>,
    pub dependants: Vec<Id<Element>>,
    pub unresolved_count: usize,
}

pub struct Module {
    pub local: UnsafeCell<ModuleLocal>,
    pub root_scope: Option<Id<Element>>,
    pub file: Option<FileId>,
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("local", unsafe { self.local.as_ref_unchecked() })
            .field("root_scope", &self.root_scope)
            .finish()
    }
}

impl ModuleLocal {
    pub fn is_resolved(&self) -> bool {
        self.unresolved_count == 0
    }
}

impl Module {
    pub fn new(authored: Option<ScopeAuthored>, resolved: bool, file: Option<FileId>) -> Self {
        Self {
            local: UnsafeCell::new(ModuleLocal {
                pools: Default::default(),
                authored,
                dependants: Default::default(),
                unresolved_count: if resolved { 0 } else { 1 },
            }),
            root_scope: Default::default(),
            file,
        }
    }
}

new_key_type! {pub struct ModuleId;}
