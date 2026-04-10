use enum_extract_macro::EnumExtract;
use smallvec::SmallVec;
use std::sync::OnceLock;
use type_sitter::UntypedNode;

use crate::{
    interpreter::{
        Id, Managed, Owner, diagnose::Diagnostic, expr::Expr, module::ModuleId, run::Runner,
        scope::Scope, value::ValueStorage,
    },
    utils::{concurrent_string_interner::StringId, moss, unsafe_cell::UnsafeCell},
};

#[derive(Clone, Copy, Debug, EnumExtract)]
pub enum ElementKey {
    Name(StringId),
    Effect,
    Temp,
}

#[derive(Debug)]
pub struct ElementLocal {
    pub expr: Option<Expr>,
    pub value: Option<ValueStorage>,
    pub dependency_count: i64,
    pub dependants: SmallVec<[Dependant; 4]>,
    pub diagnoistics: Vec<Diagnostic>,
    pub is_running: bool,
    pub runner: Option<Runner>,
}

impl ElementLocal {
    pub fn is_resolved(&self) -> bool {
        self.value.is_some()
    }
    pub fn get_resolved(&self) -> Option<ValueStorage> {
        self.value
    }
}

#[derive(Debug)]
pub struct Element {
    pub key: ElementKey,
    pub source: Option<ElementSource>,
    pub value: OnceLock<ValueStorage>,
    pub local: UnsafeCell<ElementLocal>,
    pub owner: Owner,
}

impl Managed for Element {
    const NAME: &str = "Element";

    type Local = ElementLocal;

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

impl Element {
    pub fn new<'tree>(key: ElementKey, owner: Owner) -> Self {
        Self {
            key,
            value: Default::default(),
            source: None,
            owner,
            local: UnsafeCell::new(ElementLocal {
                expr: None,
                value: None,
                dependency_count: 0,
                dependants: Default::default(),
                diagnoistics: Default::default(),
                is_running: false,
                runner: None,
            }),
        }
    }
    pub fn is_resolved(&self) -> bool {
        self.value.get().is_some()
    }
    pub fn get_resolved(&self) -> Option<ValueStorage> {
        self.value.get().copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElementSource {
    pub value_source: moss::Value<'static>,
    pub key_source: Option<moss::Name<'static>>,
    pub scope: Id<Scope>,
}

#[derive(Debug)]
pub enum ElementAuthored<'a> {
    Source {
        source: ElementSource,
        scope: &'a mut Scope,
    },
    Expr(Expr),
    Value(ValueStorage),
}

#[derive(Clone, Copy, Debug)]
pub struct Dependant {
    pub element_id: Id<Element>,
    pub source: Option<UntypedNode<'static>>,
}

#[derive(Debug)]
pub struct ElementDescriptor {
    pub key: ElementKey,
    pub value: ValueStorage,
}
