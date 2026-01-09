use std::collections::HashMap;

use slotmap::new_key_type;

use crate::{
    interpreter::{
        InModuleId, InterpreterLike,
        diagnose::Diagnostic,
        element::{InModuleElementId, RemoteInModuleElementId},
        file::FileId,
        module::ModuleId,
    },
    new_type,
    utils::{concurrent_string_interner::StringId, moss},
};

new_type!(
    #[derive(Clone, Copy,PartialEq,Debug)]
    pub RemoteInModuleScopeId = usize
);

#[derive(Clone, Copy, Debug)]
pub enum ScopeSource {
    Scope(moss::Scope<'static>),
    File(moss::SourceFile<'static>),
}

#[derive(Clone, Copy, Debug)]
pub struct ScopeAuthored {
    pub source: ScopeSource,
    pub file: FileId,
}

new_type!(
    #[derive(Clone, Copy,PartialEq,Debug)]
    pub LocalInModuleScopeId = usize
);

#[derive(Clone, Copy, Debug)]
pub struct LocalScopeId {
    pub in_module: LocalInModuleScopeId,
    pub module: ModuleId,
}

#[derive(Clone, Copy)]
pub struct RemoteScopeId {
    pub in_module: RemoteInModuleScopeId,
    pub module: ModuleId,
}

#[derive(Clone, Copy, Debug)]
pub struct ScopeId {
    pub local: LocalScopeId,
    pub remote: Option<RemoteInModuleScopeId>,
}

impl ScopeId {
    pub fn get_remote(&self) -> Option<RemoteScopeId> {
        Some(RemoteScopeId {
            in_module: self.remote?,
            module: self.local.module,
        })
    }
    pub fn from_local(interpreter: &(impl InterpreterLike + ?Sized), local: LocalScopeId) -> Self {
        let scope = interpreter.get_scope(local);
        Self {
            local,
            remote: scope.remote_id,
        }
    }
    pub fn get_module(&self) -> ModuleId {
        self.local.module
    }
}

impl InModuleId for LocalInModuleScopeId {
    type GlobalId = LocalScopeId;

    fn global(self, module: ModuleId) -> Self::GlobalId {
        Self::GlobalId {
            in_module: self,
            module,
        }
    }
}

#[derive(Debug)]
pub struct ScopeRemote {
    pub elements: HashMap<StringId, RemoteInModuleElementId>,
    pub parent: Option<RemoteInModuleScopeId>,
    pub local_id: LocalInModuleScopeId,
}

#[derive(Debug)]
pub struct Scope {
    pub elements: HashMap<StringId, InModuleElementId>,
    pub parent: Option<LocalInModuleScopeId>,
    pub children: Vec<LocalInModuleScopeId>,
    pub authored: Option<ScopeAuthored>,
    pub remote_id: Option<RemoteInModuleScopeId>,
    pub diagnoistics: Vec<Diagnostic>,
    pub module: ModuleId,
}

impl Scope {
    pub fn new(
        parent: Option<LocalInModuleScopeId>,
        authored: Option<ScopeAuthored>,
        module: ModuleId,
    ) -> Self {
        Self {
            elements: Default::default(),
            parent,
            children: Default::default(),
            authored,
            remote_id: None,
            diagnoistics: Default::default(),
            module,
        }
    }
    pub fn get_file(&self) -> Option<FileId> {
        Some(self.authored?.file)
    }
}
