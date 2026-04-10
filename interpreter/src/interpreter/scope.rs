use std::collections::HashMap;

use crate::{
    interpreter::{
        Id, Managed, Owner, diagnose::Diagnostic, element::Element, file::FileId, module::ModuleId,
    },
    utils::{concurrent_string_interner::StringId, moss, unsafe_cell::UnsafeCell},
};

#[derive(Debug)]
pub struct ScopeLocal {
    pub children: Vec<Id<Scope>>,
    pub diagnoistics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct Scope {
    pub elements: HashMap<StringId, Id<Element>>,
    pub temp_elements: Vec<Id<Element>>,
    pub parent: Option<Id<Scope>>,
    pub authored: Option<ScopeAuthored>,
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
        authored: Option<ScopeAuthored>,
        owner: Owner,
        complete: Id<Element>,
    ) -> Self {
        Self {
            elements: Default::default(),
            temp_elements: Default::default(),
            parent,
            authored,
            owner,
            local: UnsafeCell::new(ScopeLocal {
                children: Default::default(),
                diagnoistics: Default::default(),
            }),
            effects: Default::default(),
            complete,
        }
    }
    pub fn get_file(&self) -> Option<FileId> {
        Some(self.authored?.file)
    }
    pub fn visible_elements(&self) -> impl Iterator<Item = Id<Element>> {
        self.elements.values().chain(self.effects.iter()).copied()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScopeAuthored {
    pub source: Option<moss::ScopeContent<'static>>,
    pub file: FileId,
}
