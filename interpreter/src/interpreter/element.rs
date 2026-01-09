use crossbeam::atomic::AtomicCell;
use smallvec::SmallVec;
use type_sitter::UntypedNode;

use crate::{
    in_module_id,
    interpreter::{
        InModuleId,
        diagnose::Diagnostic,
        file::FileId,
        module::ModuleId,
        scope::InModuleScopeId,
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
    pub fn new(local_id: InModuleElementId, value: TypedValue,resolved:bool) -> Self {
        Self {
            cell: AtomicCell::new(ElementRemoteCell {
                value,
                resolved,
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
    pub scope: InModuleScopeId,
    pub dependency_count: i64,
    pub dependants: SmallVec<[Dependant; 4]>,
    pub resolved: bool,
    pub authored: Option<ElementSource>,
    pub remote_id: Option<RemoteInModuleElementId>,
    pub diagnoistics: Vec<Diagnostic>,
}

impl Element {
    pub fn new<'tree>(key: ElementKey, scope: InModuleScopeId) -> Self {
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

#[derive(Debug)]
pub struct ElementSource {
    pub value_source: moss::Value<'static>,
    pub key_source: Option<moss::Name<'static>>,
}

#[derive(Debug)]
pub enum ElementAuthored {
    Source { source: ElementSource, file: FileId },
    Value { value: TypedValue },
}

#[derive(Clone, Copy, Debug)]
pub struct Dependant {
    pub element_id: ElementId,
    pub source: UntypedNode<'static>,
}

#[derive(Debug)]
pub struct ElementDescriptor {
    pub key: ElementKey,
    pub value: TypedValue,
}
