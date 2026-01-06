use crossbeam::atomic::AtomicCell;
use slotmap::new_key_type;
use smallvec::SmallVec;
use type_sitter::UntypedNode;

use crate::{
    interpreter::{
        LocalId,
        diagnose::Diagnostic,
        module::ModuleId,
        scope::LocalInModuleScopeId,
        value::{TypedValue, Value},
    },
    utils::{concurrent_string_interner::StringId, moss},
};

#[derive(Clone, Copy, Debug)]
pub enum ElementKey {
    Name(StringId),
    Temp,
}

new_key_type! {pub struct LocalInModuleElementId;}

#[derive(Clone, Copy, Hash, PartialEq)]
pub struct LocalElementId {
    pub in_module: LocalInModuleElementId,
    pub module: ModuleId,
}

#[derive(Clone, Copy, Hash, PartialEq)]
pub struct RemoteElementId {
    pub in_module: usize,
    pub module: ModuleId,
}

#[derive(Clone, Copy, Hash, PartialEq)]
pub enum ElementId {
    Local(LocalElementId),
    Remote(RemoteElementId),
}

impl LocalId for LocalInModuleElementId {
    type GlobalId = LocalElementId;

    fn global(self, module: ModuleId) -> Self::GlobalId {
        Self::GlobalId {
            in_module: self,
            module,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ElementRemoteMut {
    pub value: TypedValue,
    pub resolved: bool,
}

pub struct ElementRemote {
    pub r#mut: AtomicCell<ElementRemoteMut>,
    pub local_id: LocalInModuleElementId,
}

impl ElementRemote {
    pub fn new(local_id: LocalInModuleElementId) -> Self {
        Self {
            r#mut: AtomicCell::new(ElementRemoteMut {
                value: TypedValue::err(),
                resolved: false,
            }),
            local_id,
        }
    }
}

pub struct Element {
    pub key: ElementKey,
    pub resolved_value: TypedValue,
    pub raw_value: Value,
    pub scope: LocalInModuleScopeId,
    pub dependency_count: i64,
    pub dependants: SmallVec<[Dependant; 4]>,
    pub resolved: bool,
    pub authored: Option<ElementAuthored>,
    pub remote_id: Option<usize>,
    pub diagnoistics: Vec<Diagnostic>,
}

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

#[derive(Clone, Copy)]
pub struct Dependant {
    pub element_id: LocalElementId,
    pub node: UntypedNode<'static>,
}
