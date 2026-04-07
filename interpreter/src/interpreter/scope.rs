use std::collections::HashMap;

use type_sitter::{Node, UntypedNode};

use crate::{
    interpreter::{
        Id, Managed, Owner, diagnose::Diagnostic, element::Element, file::FileId,
        function::Function, module::ModuleId,
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
    pub module: ModuleId,
    pub local: UnsafeCell<ScopeLocal>,
    pub depth: usize,
    pub effects: Vec<Id<Element>>,
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

    type Onwer = Self;

    fn get_owner(&self) -> super::Owner<Self::Onwer>
    where
        Self: Sized,
    {
        Owner::Module(self.module)
    }
}

impl Scope {
    pub fn new(
        parent: Option<Id<Scope>>,
        authored: Option<ScopeAuthored>,
        module: ModuleId,
        depth: usize,
    ) -> Self {
        Self {
            elements: Default::default(),
            temp_elements: Default::default(),
            parent,
            authored,
            module,
            local: UnsafeCell::new(ScopeLocal {
                children: Default::default(),
                diagnoistics: Default::default(),
            }),
            depth,
            effects: Default::default(),
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
