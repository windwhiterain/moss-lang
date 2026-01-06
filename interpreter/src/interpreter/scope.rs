use std::collections::HashMap;

use slotmap::new_key_type;

use crate::{
    interpreter::{
        InterpreterLike, LocalId, diagnose::Diagnostic, element::LocalInModuleElementId,
        file::FileId, module::ModuleId,
    },
    utils::{concurrent_string_interner::StringId, moss},
};

#[derive(Clone, Copy)]
pub enum ScopeSource {
    Scope(moss::Scope<'static>),
    File(moss::SourceFile<'static>),
}

#[derive(Clone, Copy)]
pub struct ScopeAuthored {
    pub source: ScopeSource,
    pub file: FileId,
}

new_key_type! {pub struct LocalInModuleScopeId;}

#[derive(Clone, Copy, Hash)]
pub struct LocalScopeId {
    pub in_module: LocalInModuleScopeId,
    pub module: ModuleId,
}

#[derive(Clone, Copy, Hash)]
pub struct RemoteScopeId {
    pub in_module: usize,
    pub module: ModuleId,
}

#[derive(Clone, Copy, Hash)]
pub struct ScopeId {
    pub local: LocalScopeId,
    pub remote: Option<usize>,
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

impl LocalId for LocalInModuleScopeId {
    type GlobalId = LocalScopeId;

    fn global(self, module: ModuleId) -> Self::GlobalId {
        Self::GlobalId {
            in_module: self,
            module,
        }
    }
}

pub struct ScopeRemote {
    pub elements: HashMap<StringId, usize>,
    pub parent: Option<usize>,
    pub local_id: LocalInModuleScopeId,
}

pub struct Scope {
    pub elements: HashMap<StringId, LocalInModuleElementId>,
    pub parent: Option<LocalInModuleScopeId>,
    pub children: Vec<LocalInModuleScopeId>,
    pub authored: Option<ScopeAuthored>,
    pub remote_id: Option<usize>,
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
