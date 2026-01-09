use crossbeam::atomic::AtomicCell;
use slotmap::new_key_type;
use smallvec::SmallVec;
use type_sitter::UntypedNode;

use crate::{
    in_module_id,
    interpreter::{
        InModuleId,
        diagnose::Diagnostic,
        module::ModuleId,
        scope::LocalInModuleScopeId,
        value::{TypedValue, Value},
    },
    new_type,
    utils::{concurrent_string_interner::StringId, moss},
};

#[derive(Clone, Copy, Debug)]
pub enum ElementKey {
    Name(StringId),
    Temp,
}

new_type! {
    #[derive(Clone,Copy,PartialEq,Debug)]
    pub InModuleElementId = usize
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ElementId {
    pub in_module: InModuleElementId,
    pub module: ModuleId,
}

in_module_id! {InModuleElementId,ElementId}

new_type! {
    #[derive(Clone,Copy,PartialEq,Debug)]
    pub RemoteInModuleElementId = usize
}

#[derive(Clone, Copy, PartialEq)]
pub struct RemoteElementId {
    pub in_module: RemoteInModuleElementId,
    pub module: ModuleId,
}

in_module_id!(RemoteInModuleElementId, RemoteElementId);

#[derive(Clone, Copy, PartialEq)]
pub enum ConcurrentElementId {
    Local(ElementId),
    Remote(RemoteElementId),
}

#[derive(Clone, Copy, Debug)]
pub struct ElementRemoteCell {
    pub value: TypedValue,
    pub resolved: bool,
}

#[derive(Debug)]
pub struct ElementRemote {
    pub cell: AtomicCell<ElementRemoteCell>,
    pub local_id: InModuleElementId,
}

impl ElementRemote {
    pub fn new(local_id: InModuleElementId) -> Self {
        Self {
            cell: AtomicCell::new(ElementRemoteCell {
                value: TypedValue::err(),
                resolved: false,
            }),
            local_id,
        }
    }
}

#[derive(Debug)]
pub struct Element {
    pub key: ElementKey,
    pub resolved_value: TypedValue,
    pub raw_value: Value,
    pub scope: LocalInModuleScopeId,
    pub dependency_count: i64,
    pub dependants: SmallVec<[Dependant; 4]>,
    pub resolved: bool,
    pub authored: Option<ElementAuthored>,
    pub remote_id: Option<RemoteInModuleElementId>,
    pub diagnoistics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct ElementAuthored {
    pub value_node: moss::Value<'static>,
    pub key_node: Option<moss::Name<'static>>,
}

impl Element {
    pub fn new<'tree>(key: ElementKey, scope: LocalInModuleScopeId) -> Self {
        Self {
            key,
            resolved_value: TypedValue::err(),
            raw_value: Value::Err,
            scope,
            dependency_count: 0,
            dependants: Default::default(),
            authored: None,
            resolved: false,
            remote_id: None,
            diagnoistics: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Dependant {
    pub element_id: ElementId,
    pub node: UntypedNode<'static>,
}

pub struct Depend {
    pub dependant: ElementId,
    pub dependency: ElementId,
    pub node: UntypedNode<'static>,
}
