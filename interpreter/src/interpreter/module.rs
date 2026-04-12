use crate::{
    gen_pools,
    interpreter::{
        Id,
        error::Error,
        file::FileId,
        function::{Function, FunctionBody, Param},
        scope,
        set::Set,
    },
    utils::pool::Pool,
};
use slotmap::new_key_type;
use std::{cell::UnsafeCell, fmt::Debug};

use crate::interpreter::{element::Element, scope::Scope};

gen_pools! {
    #[derive(Debug)]
    pub Pools{scopes:Scope,elements:Element,functions:Function,params:Param,function_bodies:FunctionBody,sets:Set,errors:Error}
}

#[derive(Debug, Clone, Copy)]
pub struct Authored {
    pub source: scope::Source,
    pub file: FileId,
}

#[derive(Debug)]
pub struct ModuleLocal {
    pub pools: Pools,
    pub unresolved_count: usize,
    pub errors: Vec<Error>,
}

pub struct Module {
    pub local: UnsafeCell<ModuleLocal>,
    pub root_scope: Option<Id<Element>>,
    pub authored: Option<Authored>,
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
    pub fn new(authored: Option<Authored>, resolved: bool) -> Self {
        Self {
            local: UnsafeCell::new(ModuleLocal {
                pools: Default::default(),
                unresolved_count: if resolved { 0 } else { 1 },
                errors: Default::default(),
            }),
            root_scope: Default::default(),
            authored,
        }
    }
}

new_key_type! {pub struct ModuleId;}
