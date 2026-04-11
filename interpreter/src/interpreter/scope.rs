use std::collections::HashMap;

use crate::{
    interpreter::{
        Id, Managed, Owner, element::Element, error::Kind, file::FileId, module::ModuleId,
    },
    utils::{concurrent_string_interner::StringId, moss, unsafe_cell::UnsafeCell},
};

pub type Source = moss::ScopeContent<'static>;

#[derive(Debug)]
pub struct ScopeLocal {
    pub children: Vec<Id<Scope>>,
    pub diagnoistics: Vec<Kind>,
}

#[derive(Debug)]
pub struct Scope {
    pub elements: HashMap<StringId, Id<Element>>,
    pub sourced_elements: Vec<Id<Element>>,
    pub parent: Option<Id<Scope>>,
    pub source: Option<Source>,
    pub local: UnsafeCell<ScopeLocal>,
    pub effects: Vec<Id<Element>>,
    pub complete: Id<Element>,
    pub owner: Owner,
}

impl Managed for Scope {
    const NAME: &str = "Scope";

    type Local = ScopeLocal;

    fn get_local(&self) -> &UnsafeCell<Self::Local> {
        &self.local
    }

    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local> {
        &mut self.local
    }

    fn get_module<IP: super::InterpreterLike>(&self, ip: &IP) -> ModuleId
    where
        Self: Sized,
    {
        self.owner.module(ip)
    }
}

impl Scope {
    pub fn new(
        parent: Option<Id<Scope>>,
        source: Option<Source>,
        owner: Owner,
        complete: Id<Element>,
    ) -> Self {
        Self {
            elements: Default::default(),
            sourced_elements: Default::default(),
            parent,
            source,
            owner,
            local: UnsafeCell::new(ScopeLocal {
                children: Default::default(),
                diagnoistics: Default::default(),
            }),
            effects: Default::default(),
            complete,
        }
    }
    pub fn visible_elements(&self) -> impl Iterator<Item = Id<Element>> {
        self.elements.values().chain(self.effects.iter()).copied()
    }
}
