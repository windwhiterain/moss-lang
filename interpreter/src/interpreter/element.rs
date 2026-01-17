use smallvec::SmallVec;
use std::{cell::UnsafeCell, sync::OnceLock};
use type_sitter::UntypedNode;

use crate::{
    interpreter::{
        Id, Managed, Owner, diagnose::Diagnostic, file::FileId, scope::Scope, value::{TypedValue, Value}
    },
    utils::{concurrent_string_interner::StringId, moss},
};

#[derive(Clone, Copy, Debug)]
pub enum ElementKey {
    Name(StringId),
    Temp,
}

#[derive(Debug)]
pub struct ElementLocal {
    pub value: TypedValue,
    pub resolved: bool,
    pub dependency_count: i64,
    pub dependants: SmallVec<[Dependant; 4]>,
    pub diagnoistics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct Element {
    pub key: ElementKey,
    pub scope: Id<Scope>,
    pub source: Option<ElementSource>,
    pub value: OnceLock<TypedValue>,
    pub local: UnsafeCell<ElementLocal>,
}

impl Managed for Element{
    const NAME: &str = "Element";
    
    type Local = ElementLocal;
    
    fn get_local(&self)->&UnsafeCell<Self::Local> {
        &self.local
    }
    
    fn get_local_mut(&mut self)->&mut UnsafeCell<Self::Local> {
        &mut self.local
    }
    type Onwer = Scope;
    
    fn get_owner(&self)->super::Owner<Self::Onwer> where Self: Sized {
        Owner::Managed(self.scope)
    }
}



impl Element {
    pub fn new<'tree>(key: ElementKey, scope: Id<Scope>, source: Option<ElementSource>) -> Self {
        Self {
            key,
            value: Default::default(),
            scope,
            source,
            local: UnsafeCell::new(ElementLocal {
                value: TypedValue::err(),
                dependency_count: 0,
                dependants: Default::default(),
                resolved: false,
                diagnoistics: Default::default(),
            }),
        }
    }
}

#[derive(Debug,Clone,Copy)]
pub struct ElementSource {
    pub value_source: moss::Value<'static>,
    pub key_source: Option<moss::Name<'static>>,
}

#[derive(Debug,Clone,Copy)]
pub enum ElementAuthored {
    Source { source: ElementSource, file: FileId },
    Value { value: TypedValue },
}

#[derive(Clone, Copy, Debug)]
pub struct Dependant {
    pub element_id: Id<Element>,
    pub source: UntypedNode<'static>,
}

#[derive(Debug)]
pub struct ElementDescriptor {
    pub key: ElementKey,
    pub value: TypedValue,
}
