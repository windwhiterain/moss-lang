use enum_extract_macro::EnumExtract;
use smallvec::SmallVec;
use std::sync::OnceLock;
use type_sitter::UntypedNode;

use crate::{
    interpreter::{
        Id, Managed, Owner, diagnose::Diagnostic, expr::Expr, scope::Scope, value::Value,
    },
    utils::{concurrent_string_interner::StringId, moss, unsafe_cell::UnsafeCell},
};

#[derive(Clone, Copy, Debug,EnumExtract)]
pub enum ElementKey {
    Name(StringId),
    Temp,
}

#[derive(Debug)]
pub struct ElementLocal {
    pub expr: Option<Expr>,
    pub value: Option<Value>,
    pub dependency_count: i64,
    pub dependants: SmallVec<[Dependant; 4]>,
    pub diagnoistics: Vec<Diagnostic>,
    pub is_running: bool,
}

impl ElementLocal {
    pub fn is_resolved(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Debug)]
pub struct Element {
    pub key: ElementKey,
    pub scope: Id<Scope>,
    pub source: Option<ElementSource>,
    pub value: OnceLock<Value>,
    pub local: UnsafeCell<ElementLocal>,
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
    type Onwer = Scope;

    fn get_owner(&self) -> super::Owner<Self::Onwer>
    where
        Self: Sized,
    {
        Owner::Managed(self.scope)
    }
}

impl Element {
    pub fn new<'tree>(key: ElementKey, scope: Id<Scope>) -> Self {
        Self {
            key,
            value: Default::default(),
            scope,
            source: None,
            local: UnsafeCell::new(ElementLocal {
                expr: None,
                value: None,
                dependency_count: 0,
                dependants: Default::default(),
                diagnoistics: Default::default(),
                is_running: false,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElementSource {
    pub value_source: moss::Value<'static>,
    pub key_source: Option<moss::Name<'static>>,
}

#[derive(Debug, Clone)]
pub enum ElementAuthored {
    Source(ElementSource),
    Expr(Expr),
    Value(Value),
}

#[derive(Clone, Copy, Debug)]
pub struct Dependant {
    pub element_id: Id<Element>,
    pub source: Option<UntypedNode<'static>>,
}

#[derive(Debug)]
pub struct ElementDescriptor {
    pub key: ElementKey,
    pub value: Value,
}
