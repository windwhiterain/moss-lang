use crate::erase_struct;
use crate::interpreter::diagnose::Diagnostic;
use crate::utils::async_lockfree_stack::Stack;
use crate::utils::concurrent_string_interner::ConcurentInterner;
use crate::utils::erase;
use crate::utils::erase_mut;
use crate::utils::moss::BuiltinChild;
use crossbeam::atomic::AtomicCell;
use sharded_slab::Slab;
use slotmap::SecondaryMap;
use slotmap::SlotMap;
use slotmap::new_key_type;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::mem;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread::available_parallelism;
use strum::EnumIter;
use tokio::sync::Notify;
use tokio::task::JoinSet;

use crate::utils::moss;
use tree_sitter::Parser;
use type_sitter::HasChild;
pub use type_sitter::Node;
use type_sitter::NodeResult;
pub use type_sitter::UntypedNode;
pub type Tree = type_sitter::Tree<moss::SourceFile<'static>>;

pub mod diagnose;

pub trait LocalId {
    type GlobalId;
    fn global(self, module: ModuleId) -> Self::GlobalId;
}

pub struct ModuleRemote {
    pub scopes: Slab<ScopeRemote>,
    pub elements: Slab<ElementRemote>,
    pub root_scope: OnceLock<usize>,
}

pub struct ModuleCell {
    pub scopes: SlotMap<LocalInModuleScopeId, Scope>,
    pub elements: SlotMap<LocalInModuleElementId, Element>,
    pub authored: Option<ScopeAuthored>,
    pub dependants: Vec<LocalElementId>,
    pub root_scope: OnceCell<LocalInModuleScopeId>,
}

impl ModuleCell {
    fn has_parsed(&self) -> bool {
        self.root_scope.get().is_some()
    }
}

pub struct Module {
    pub cell: UnsafeCell<ModuleCell>,
    pub remote: ModuleRemote,
}

impl Module {
    fn new(source: ScopeAuthored) -> Self {
        Self {
            cell: UnsafeCell::new(ModuleCell {
                scopes: Default::default(),
                elements: Default::default(),
                authored: Some(source),
                dependants: Default::default(),
                root_scope: Default::default(),
            }),
            remote: ModuleRemote {
                scopes: Default::default(),
                elements: Default::default(),
                root_scope: Default::default(),
            },
        }
    }
}

new_key_type! {pub struct ModuleId;}

pub struct Thread {
    pub r#mut: UnsafeCell<ThreadMut>,
    pub remote: ThreadRemote,
}

impl Thread {
    pub fn new(module_ids: Vec<ModuleId>) -> Self {
        Self {
            r#mut: UnsafeCell::new(ThreadMut {
                modules: module_ids,
                add_module_delay: AddModuleDelay {
                    files: Default::default(),
                    scopes: Default::default(),
                },
            }),
            remote: ThreadRemote {
                channel: Arc::new(Stack::new()),
            },
        }
    }
}

pub struct ThreadMut {
    pub modules: Vec<ModuleId>,
    pub add_module_delay: AddModuleDelay,
}

pub struct ThreadRemote {
    pub channel: Arc<Stack<Signal>>,
}

pub enum Signal {
    Depend(Depend),
    Resolve(LocalElementId),
}

new_key_type! {pub struct ThreadId;}

pub struct Depend {
    pub dependant: LocalElementId,
    pub dependency: LocalElementId,
    pub node: UntypedNode<'static>,
}

pub struct Interpreter {
    pub workspace_path: PathBuf,
    pub source_path: PathBuf,
    pub strings: StringInterner,
    pub files: SlotMap<FileId, File>,
    pub uri2file: hashbrown::HashMap<PathBuf, FileId>,
    pub root_scope: Option<LocalInModuleScopeId>,
    pub modules: SlotMap<ModuleId, Module>,
    pub remote: InterpreterRemote,
    pub single_thread: bool,
}

unsafe impl Sync for Interpreter {}

impl Interpreter {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            workspace_path: workspace_path.clone(),
            source_path: workspace_path.join("src"),
            strings: StringInterner::new(),
            files: Default::default(),
            uri2file: Default::default(),
            root_scope: None,
            modules: Default::default(),
            remote: InterpreterRemote {
                module2thread: Default::default(),
                threads: Default::default(),
                strings: ConcurentInterner::new(),
            },
            single_thread: true,
        }
    }
    pub fn clear(&mut self) {
        self.root_scope = None;
        for file in self.files.values_mut() {
            file.is_module = None;
        }
        self.modules.clear();
        self.remote.module2thread.clear();
        self.remote.threads.clear();
    }
    pub fn assert_single_thread(&self) {
        assert!(self.single_thread)
    }
    pub fn get_file_mut(&mut self, id: FileId) -> &mut File {
        &mut self.files[id]
    }
    pub fn find_file(&self, path: impl AsRef<Path>) -> Option<FileId> {
        self.uri2file.get(path.as_ref()).copied()
    }
    pub fn find_or_add_file(&mut self, path: Cow<PathBuf>) -> FileId {
        match self.uri2file.raw_entry_mut().from_key(path.as_path()) {
            hashbrown::hash_map::RawEntryMut::Occupied(raw_occupied_entry_mut) => {
                *raw_occupied_entry_mut.get()
            }
            hashbrown::hash_map::RawEntryMut::Vacant(raw_vacant_entry_mut) => {
                let path = path.to_path_buf();
                let file = File::new(path.clone());
                let file_id = self.files.insert(file);
                raw_vacant_entry_mut.insert(path, file_id);
                file_id
            }
        }
    }
    pub fn add_module_raw(&mut self, source: ScopeAuthored) -> ModuleId {
        let id = self.modules.insert(Module::new(source));
        match source.source {
            ScopeSource::Scope(scope) => {}
            ScopeSource::File(source_file) => self.get_file_mut(source.file).is_module = Some(id),
        };
        id
    }
    pub fn add_module(&mut self, authored: ModuleAuthored) -> ModuleId {
        match authored {
            ModuleAuthored::File { path } => {
                let file_id = self.find_or_add_file(Cow::Owned(path));
                let file = erase_mut(self).get_file(file_id);
                let source = ScopeSource::File(file.tree.root_node().unwrap());
                let module = erase_mut(self).add_module_raw(ScopeAuthored {
                    source,
                    file: file_id,
                });
                module
            }
            ModuleAuthored::Scope {
                file: file_id,
                source,
            } => {
                let source = ScopeSource::Scope(source);
                let module = erase_mut(self).add_module_raw(ScopeAuthored {
                    source,
                    file: file_id,
                });

                module
            }
        }
    }
    pub fn create_threads(&mut self) {
        self.assert_single_thread();
        self.remote.strings.sync_from(&self.strings);
        let mut thread_num: usize = available_parallelism().unwrap().into();
        let mut module_num = self.modules.len();
        let mut modules = self.modules.iter_mut();
        loop {
            let mut module_per_thread = module_num.div_ceil(thread_num);
            if module_per_thread == 0 {
                break;
            }
            let mut module_ids = vec![];
            loop {
                let (id, module) = modules.next().unwrap();
                module_ids.push(id);
                let thread_id = self.remote.threads.insert(Thread::new(Default::default()));
                self.remote.module2thread.insert(id, Some(thread_id));
                module_per_thread -= 1;
                module_num -= 1;
                if module_per_thread == 0 {
                    let thread = &mut self.remote.threads[thread_id];
                    thread.r#mut.get_mut().modules = module_ids;
                    break;
                }
            }
            thread_num -= 1;
        }
        self.single_thread = false;
    }
    pub async fn sync(&mut self) {
        let wait_counter = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let mut set = JoinSet::new();
        for thread in self.remote.threads.keys() {
            let mut thread_interpreter = ThreadInterpreter {
                interpreter: erase(self),
                thread,
            };
            let wait_counter = wait_counter.clone();
            let notify = notify.clone();
            set.spawn(async move { thread_interpreter.run(wait_counter, notify).await });
        }
        log::error!("join_all(");
        set.join_all().await;
        log::error!("join_all)");
        self.strings.sync_from(&self.remote.strings);
        self.single_thread = true;
        for thread_id in erase(self).remote.threads.keys() {
            let thread_remote = erase(self).get_thread_remote(thread_id);
            loop {
                let Some(signal) = thread_remote.channel.pop() else {
                    break;
                };
                match signal {
                    Signal::Depend(depend) => {
                        self.depend_element(
                            depend.dependant,
                            ElementId::Local(depend.dependency),
                            depend.node,
                            false,
                        );
                    }
                    Signal::Resolve(local_element_id) => {
                        self.resolve_element(local_element_id);
                    }
                }
            }
        }
        for thread_id in erase(self).remote.threads.keys() {
            let thread = erase_mut(self).get_thread_mut(thread_id);
            for (path, dependants) in mem::take(&mut thread.add_module_delay.files) {
                let file_id = self.find_or_add_file(Cow::Owned(path));
                let file = self.get_file(file_id);
                let module_id = if let Some(module) = file.is_module {
                    module
                } else {
                    self.add_module_raw(ScopeAuthored {
                        source: ScopeSource::File(erase_struct!(file.tree.root_node().unwrap())),
                        file: file_id,
                    })
                };
                let module = self.get_module_mut(module_id);
                for dependant in dependants.iter().copied() {
                    module.dependants.push(dependant);
                }
            }
            for scope in mem::take(&mut thread.add_module_delay.scopes) {
                let module_id = self.add_module_raw(ScopeAuthored {
                    source: ScopeSource::Scope(scope.scope),
                    file: scope.file,
                });
                let module = self.get_module_mut(module_id);
                module.dependants.push(scope.element);
            }
        }
    }
    pub async fn run(&mut self) {
        self.create_threads();
        self.sync().await;
    }
}

pub struct InterpreterRemote {
    pub module2thread: SecondaryMap<ModuleId, Option<ThreadId>>,
    pub threads: SlotMap<ThreadId, Thread>,
    pub strings: ConcurrentStringInterner,
}

pub struct ThreadInterpreter<IP: Deref<Target = Interpreter>> {
    pub interpreter: IP,
    pub thread: ThreadId,
}

impl<IP: Deref<Target = Interpreter>> ThreadInterpreter<IP> {
    fn is_module_local(&self, module: ModuleId) -> bool {
        Some(self.thread) == self.interpreter.remote.module2thread[module]
    }
    fn is_module_remote(&self, module: ModuleId) -> bool {
        if let Some(id) = self.interpreter.remote.module2thread[module] {
            id != self.thread
        } else {
            false
        }
    }
    async fn run(&mut self, wait_counter: Arc<AtomicUsize>, notify: Arc<Notify>) {
        for module_id in erase_mut(self)
            .get_thread_mut(self.thread)
            .modules
            .iter()
            .copied()
        {
            let module = erase_mut(self.get_module_mut(module_id));
            if !module.has_parsed() {
                self.run_module(module_id);
                for dependant in mem::take(&mut module.dependants) {
                    self.resolve_element(dependant);
                }
            }
        }
        let thread = erase(self.get_thread_remote(self.thread));
        loop {
            if let Some(signal) = thread.channel.pop() {
                match signal {
                    Signal::Depend(depend) => {
                        self.depend_element(
                            depend.dependant,
                            ElementId::Local(depend.dependency),
                            depend.node,
                            false,
                        );
                    }
                    Signal::Resolve(local_element_id) => {
                        self.resolve_element(local_element_id);
                    }
                }
            } else {
                let wait_count = wait_counter.fetch_add(1, Ordering::Relaxed) + 1;
                if wait_count >= self.interpreter.remote.threads.len() {
                    notify.notify_waiters();
                    return;
                }
                tokio::select! {
                    signal=thread.channel.async_pop()=>{
                        wait_counter.fetch_sub(1, Ordering::Relaxed);
                        match signal {
                            Signal::Depend(depend) => {
                                self.get_element_mut(depend.dependency)
                                    .dependants
                                    .push(Dependant {
                                        element_id: depend.dependant,
                                        node: depend.node,
                                    });
                            }
                            Signal::Resolve(local_element_id) => {
                                self.resolve_element(local_element_id);
                            }
                        }
                    }
                    _=notify.notified()=>{return;}
                }
            }
        }
    }
}

pub struct AddModuleDelay {
    files: HashMap<PathBuf, Vec<LocalElementId>>,
    scopes: Vec<AddModuleDelayScope>,
}

pub struct AddModuleDelayScope {
    file: FileId,
    scope: moss::Scope<'static>,
    element: LocalElementId,
}

pub enum ModuleAuthored {
    File {
        path: PathBuf,
    },
    Scope {
        file: FileId,
        source: moss::Scope<'static>,
    },
}

pub type StringInterner = crate::utils::concurrent_string_interner::Interner;

pub type ConcurrentStringInterner = crate::utils::concurrent_string_interner::ConcurentInterner;

pub type StringId = crate::utils::concurrent_string_interner::StringId;

#[derive(Clone, Copy)]
pub enum ScopeSource {
    Scope(moss::Scope<'static>),
    File(moss::SourceFile<'static>),
}

#[derive(Clone, Copy)]
pub struct ScopeAuthored {
    source: ScopeSource,
    file: FileId,
}

new_key_type! {pub struct LocalInModuleScopeId;}

#[derive(Clone, Copy, Hash)]
pub struct LocalScopeId {
    pub in_module: LocalInModuleScopeId,
    pub module: ModuleId,
}

#[derive(Clone, Copy, Hash)]
pub struct RemoteScopeId {
    in_module: usize,
    module: ModuleId,
}

#[derive(Clone, Copy, Hash)]
pub struct ScopeId {
    local: LocalScopeId,
    remote: Option<usize>,
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
    elements: HashMap<StringId, usize>,
    parent: Option<usize>,
    local_id: LocalInModuleScopeId,
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
    fn new(
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

pub enum Location {
    Element(LocalElementId),
    Scope(LocalScopeId),
}

#[derive(Clone, Copy)]
pub struct Dependant {
    element_id: LocalElementId,
    node: UntypedNode<'static>,
}

#[derive(Clone, Copy, Debug)]
pub enum ElementKey {
    Name(StringId),
    Temp,
}

new_key_type! {pub struct LocalInModuleElementId;}

#[derive(Clone, Copy, Hash, PartialEq)]
pub struct LocalElementId {
    in_module: LocalInModuleElementId,
    module: ModuleId,
}

#[derive(Clone, Copy, Hash, PartialEq)]
pub struct RemoteElementId {
    in_module: usize,
    module: ModuleId,
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
    value: TypedValue,
    resolved: bool,
}

pub struct ElementRemote {
    r#mut: AtomicCell<ElementRemoteMut>,
    local_id: LocalInModuleElementId,
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
    fn new<'tree>(key: ElementKey, scope: LocalInModuleScopeId) -> Self {
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
pub struct TypedValue {
    pub value: Value,
    pub r#type: Value,
}

impl TypedValue {
    pub fn err() -> Self {
        Self {
            value: Value::Err,
            r#type: Value::Err,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Value {
    Int(i64),
    IntTy,
    String(StringId),
    StringTy,
    Scope(ScopeId),
    ScopeTy,
    TyTy,
    Builtin(Builtin),
    Name {
        name: StringId,
        scope: LocalScopeId,
        node: moss::Name<'static>,
    },
    Find {
        value: LocalElementId,
        key: StringId,
        key_source: moss::Name<'static>,
        source: moss::Find<'static>,
    },
    Call {
        func: LocalElementId,
        param: LocalElementId,
        source: moss::Call<'static>,
    },
    Err,
}

pub struct ContextedValue<'a, T: InterpreterLike + ?Sized> {
    pub value: &'a Value,
    pub ctx: &'a T,
}

impl<'a, T: InterpreterLike + ?Sized> fmt::Display for ContextedValue<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.value {
            Value::Int(x) => write!(f, "{x}"),
            Value::IntTy => write!(f, "Int"),
            Value::Scope(scope_id) => {
                let local_scope_id = scope_id.local;
                let collection = self.ctx.get_scope(local_scope_id);
                write!(f, "{{")?;
                for (key, element) in &collection.elements {
                    let element = self.ctx.get_element(element.global(local_scope_id.module));
                    write!(
                        f,
                        "{}: {}, ",
                        self.ctx.id2str(*key).deref(),
                        ContextedValue {
                            value: &element.resolved_value.value,
                            ctx: self.ctx
                        }
                    )?;
                }
                write!(f, "}}")
            }
            Value::ScopeTy => write!(f, "Scope"),
            Value::TyTy => write!(f, "Type"),
            Value::Builtin(builtin) => write!(f, "@{}", builtin),
            Value::Name { name, scope, .. } => {
                write!(f, "{}", self.ctx.id2str(name).deref())
            }
            Value::Find {
                value: element,
                key,
                ..
            } => {
                let element = self.ctx.get_element(element);
                write!(
                    f,
                    "{}.{}",
                    ContextedValue {
                        value: &element.resolved_value.value,
                        ctx: self.ctx
                    },
                    self.ctx.id2str(key).deref()
                )
            }
            Value::Call { func, param, .. } => {
                let func_element = self.ctx.get_element(func);
                let param_element = self.ctx.get_element(param);
                write!(
                    f,
                    "({} {})",
                    ContextedValue {
                        value: &func_element.resolved_value.value,
                        ctx: self.ctx
                    },
                    ContextedValue {
                        value: &param_element.resolved_value.value,
                        ctx: self.ctx
                    }
                )
            }
            Value::Err => write!(f, "Err"),
            Value::String(string) => {
                write!(f, "{}", self.ctx.id2str(string).deref())
            }
            Value::StringTy => write!(f, "String"),
        }
    }
}

pub struct File {
    pub text: String,
    pub parser: Parser,
    pub tree: Tree,
    pub is_module: Option<ModuleId>,
    pub path: PathBuf,
}

new_key_type! {pub struct FileId;}

impl File {
    fn new(path: PathBuf) -> Self {
        let text = fs::read_to_string(&path).unwrap();
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_moss::LANGUAGE.into())
            .unwrap();
        let tree = Tree::wrap(parser.parse(&text, None).unwrap());
        Self {
            text,
            parser,
            tree,
            is_module: None,
            path,
        }
    }
    pub fn update(&mut self) {
        self.text = fs::read_to_string(&self.path).unwrap();
        self.tree = Tree::wrap(self.parser.parse(&self.text, None).unwrap());
        self.is_module = None;
    }
}

#[derive(Clone, Copy)]
pub enum Builtin {
    If,
    Add,
    Mod,
}

impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Builtin::If => write!(f, "if"),
            Builtin::Add => write!(f, "add"),
            Builtin::Mod => write!(f, "mod"),
        }
    }
}

pub trait InterpreterLikeMut: InterpreterLike {
    fn str2id(&mut self, str: &str) -> StringId;
    fn get_node_str_id<'tree>(&mut self, node: &impl Node<'tree>, file: FileId) -> StringId {
        let str = erase(self.get_node_str(node, file));
        self.str2id(str)
    }
    fn get_thread_mut(&mut self, id: ThreadId) -> &mut ThreadMut;
    fn get_thread_mut_of(&mut self, module: ModuleId) -> Option<&mut ThreadMut>;
    fn get_module_mut(&mut self, id: ModuleId) -> &mut ModuleCell;

    fn get_element_mut(&mut self, id: LocalElementId) -> &mut Element {
        &mut self.get_module_mut(id.module).elements[id.in_module]
    }
    fn get_scope_mut(&mut self, id: LocalScopeId) -> &mut Scope {
        &mut self.get_module_mut(id.module).scopes[id.in_module]
    }

    fn add_element_raw(&mut self, element: Element, module: ModuleId) -> LocalInModuleElementId {
        self.get_module_mut(module).elements.insert(element)
    }
    fn add_scope_raw(&mut self, scope: Scope, module: ModuleId) -> LocalInModuleScopeId {
        self.get_module_mut(module).scopes.insert(scope)
    }
    fn run_module(&mut self, module_id: ModuleId) {
        let module = erase_mut(self).get_module_mut(module_id);
        if module.has_parsed() {
            return;
        }
        if let Some(authored) = module.authored {
            let root_scope = self.add_scope(None, Some(authored), module_id);
            module.root_scope.set(root_scope).unwrap();
            let remote_scope = self.publish_scope(root_scope.global(module_id));
            self.get_module_remote(module_id)
                .root_scope
                .set(remote_scope)
                .unwrap();
            for element in module.elements.keys() {
                self.run_element(element.global(module_id));
            }
        }
    }
    fn depend_module_raw(
        &mut self,
        authored: ModuleAuthored,
        element_id: LocalElementId,
    ) -> Option<ModuleId>;
    fn depend_module(
        &mut self,
        authored: ModuleAuthored,
        element_id: LocalElementId,
    ) -> Option<ModuleId> {
        if let ModuleAuthored::Scope { file, source } = authored {
            let file = self.get_file(file);
            if let Some(module) = file.is_module {
                return Some(module);
            }
        }
        self.depend_module_raw(authored, element_id)
    }

    fn depend_element(
        &mut self,
        dependant_id: LocalElementId,
        dependency_id: ElementId,
        node: UntypedNode<'static>,
        local: bool,
    );
    fn depend_element_value(
        &mut self,
        dependant_id: LocalElementId,
        dependency_id: ElementId,
        node: UntypedNode<'static>,
    ) -> Option<TypedValue> {
        let value = self.get_element_value(dependency_id);
        if value.is_none() {
            self.depend_element(dependant_id, dependency_id, node, true);
        }
        value
    }
    fn depend_child_element_value(
        &mut self,
        dependant_id: LocalElementId,
        dependency_id: LocalElementId,
    ) -> Option<TypedValue> {
        let dependency = self.get_element(dependency_id);
        let node = dependency.authored.as_ref().unwrap().value_node.upcast();
        self.depend_element_value(dependant_id, ElementId::Local(dependency_id), node)
    }
    fn resolve_element(&mut self, id: LocalElementId);

    fn grammar_error<T>(
        &mut self,
        location: Location,
        result: NodeResult<'static, T>,
    ) -> Option<T> {
        match result {
            Ok(node) => Some(node),
            Err(err) => {
                self.diagnose(
                    location,
                    Diagnostic::GrammarError {
                        source: err.node.upcast(),
                    },
                );
                None
            }
        }
    }
    fn diagnose(&mut self, location: Location, diagnoistic: Diagnostic) {
        match location {
            Location::Element(local_element_id) => {
                let element = self.get_element_mut(local_element_id);
                element.diagnoistics.push(diagnoistic);
            }
            Location::Scope(local_scope_id) => {
                let scope = self.get_scope_mut(local_scope_id);
                scope.diagnoistics.push(diagnoistic);
            }
        }
    }
    fn parse_value(
        &mut self,
        node: NodeResult<'static, moss::Value<'static>>,
        element_id: LocalElementId,
        file: FileId,
    ) -> Option<Value> {
        let node = self.grammar_error(Location::Element(element_id), node)?;
        let node_child = self.grammar_error(Location::Element(element_id), node.child())?;
        let element = erase_mut(self.get_element_mut(element_id));
        let scope_id = element.scope.global(element_id.module);
        let value = match node_child {
            moss::ValueChild::Bracket(bracket) => self
                .parse_value(bracket.value(), element_id, file)
                .unwrap_or(Value::Err),
            moss::ValueChild::Call(call) => {
                let func = self.grammar_error(Location::Element(element_id), call.func())?;
                let param = self.grammar_error(Location::Element(element_id), call.param())?;
                let func_element = self
                    .parse_element(
                        ElementKey::Temp,
                        scope_id,
                        ElementAuthored {
                            value_node: func,
                            key_node: None,
                        },
                        file,
                    )
                    .unwrap();
                let param_element = self
                    .parse_element(
                        ElementKey::Temp,
                        scope_id,
                        ElementAuthored {
                            value_node: param,
                            key_node: None,
                        },
                        file,
                    )
                    .unwrap();
                Value::Call {
                    func: func_element,
                    param: param_element,
                    source: call,
                }
            }
            moss::ValueChild::Scope(scope) => Value::Scope(ScopeId::from_local(
                erase(self),
                self.add_scope(
                    Some(scope_id.in_module),
                    Some(ScopeAuthored {
                        source: ScopeSource::Scope(scope),
                        file,
                    }),
                    element_id.module,
                )
                .global(element_id.module),
            )),
            moss::ValueChild::Find(find) => {
                let value = self.grammar_error(Location::Element(element_id), find.value())?;
                let name = self.grammar_error(Location::Element(element_id), find.name())?;
                let element = self
                    .parse_element(
                        ElementKey::Temp,
                        scope_id,
                        ElementAuthored {
                            value_node: value,
                            key_node: None,
                        },
                        file,
                    )
                    .unwrap();
                Value::Find {
                    value: element,
                    key: self.get_node_str_id(&name, file),
                    key_source: name,
                    source: find,
                }
            }
            moss::ValueChild::Int(int) => {
                Value::Int(self.get_node_str(&int, file).parse().unwrap())
            }
            moss::ValueChild::Name(name) => {
                let string_id = self.get_node_str_id(&name, file);
                Value::Name {
                    name: string_id,
                    scope: scope_id,
                    node: name,
                }
            }
            moss::ValueChild::String(string) => {
                let name = self.grammar_error(Location::Element(element_id), string.value())?;
                Value::String(self.get_node_str_id(&name, file))
            }
            moss::ValueChild::Builtin(builtin) => {
                let builtin =
                    match self.grammar_error(Location::Element(element_id), builtin.child())? {
                        BuiltinChild::BuiltinAdd(builtin_add) => Builtin::Add,
                        BuiltinChild::BuiltinIf(builtin_if) => Builtin::If,
                        BuiltinChild::BuiltinMod(builtin_mod) => Builtin::Mod,
                    };
                Value::Builtin(builtin)
            }
            _ => Value::Err,
        };
        Some(value)
    }
    fn add_element(
        &mut self,
        key: ElementKey,
        scope_id: LocalScopeId,
        value: TypedValue,
    ) -> LocalElementId {
        let scope = erase_mut(self.get_scope_mut(scope_id));
        let id = self
            .add_element_raw(Element::new(key, scope_id.in_module), scope_id.module)
            .global(scope_id.module);
        let element = erase_mut(self.get_element_mut(id));
        element.resolved_value = value;
        element.resolved = true;
        match key {
            ElementKey::Name(name) => {
                scope.elements.insert(name, id.in_module);
            }
            _ => (),
        };
        id
    }
    fn parse_element(
        &mut self,
        key: ElementKey,
        scope_id: LocalScopeId,
        authored: ElementAuthored,
        file: FileId,
    ) -> Result<LocalElementId, ()> {
        let scope = erase_mut(self.get_scope_mut(scope_id));
        let new_id = |self_: &mut Self| {
            self_
                .add_element_raw(Element::new(key, scope_id.in_module), scope_id.module)
                .global(scope_id.module)
        };
        let id = match key {
            ElementKey::Name(name) => scope
                .elements
                .entry(name)
                .or_insert_with(|| new_id(self).in_module)
                .global(scope_id.module),
            ElementKey::Temp => new_id(self),
        };
        let element = erase_mut(self.get_element_mut(id));
        if element.authored.is_some() {
            return Err(());
        }
        element.raw_value = self
            .parse_value(Ok(authored.value_node), id, file)
            .unwrap_or(Value::Err);
        element.authored = Some(authored);
        Ok(id)
    }
    fn publish_scope(&mut self, id: LocalScopeId) -> usize {
        let scope = erase_mut(self).get_scope_mut(id);
        let module = erase_mut(self).get_module_remote(id.module);
        let mut remote_elements = HashMap::<StringId, usize>::new();
        for (key, element_id) in &erase(scope).elements {
            let element = self.get_element_mut(element_id.global(id.module));
            let remote = ElementRemote::new(*element_id);
            let remote_id = module.elements.insert(remote).unwrap();
            element.remote_id = Some(remote_id);
            remote_elements.insert(*key, remote_id);
        }
        let parent = scope
            .parent
            .map(|x| self.get_scope(x.global(id.module)).remote_id)
            .flatten();
        let remote = ScopeRemote {
            elements: remote_elements,
            parent,
            local_id: id.in_module,
        };
        let remote_id = module.scopes.insert(remote).unwrap();
        scope.remote_id = Some(remote_id);
        remote_id
    }
    fn add_scope(
        &mut self,
        parent: Option<LocalInModuleScopeId>,
        authored: Option<ScopeAuthored>,
        module: ModuleId,
    ) -> LocalInModuleScopeId {
        let scope_id = self
            .add_scope_raw(Scope::new(parent, authored, module), module)
            .global(module);
        if let Some(parent) = parent {
            let parent = self.get_scope_mut(parent.global(module));
            parent.children.push(scope_id.in_module);
        }
        if let Some(authored) = authored {
            let mut cursor = erase_struct!(self.get_file(authored.file).tree.walk());

            let assigns = if let ScopeSource::Scope(scope) = authored.source {
                Some(scope.assigns(erase_mut(&mut cursor)))
            } else {
                None
            }
            .into_iter()
            .flatten()
            .chain(
                if let ScopeSource::File(source_file) = authored.source {
                    Some(source_file.assigns(&mut cursor))
                } else {
                    None
                }
                .into_iter()
                .flatten(),
            );

            for assign in assigns {
                let Some(assign) = self.grammar_error(Location::Scope(scope_id), assign) else {
                    continue;
                };

                let Some(key) = self.grammar_error(Location::Scope(scope_id), assign.key()) else {
                    continue;
                };
                let Some(value) = self.grammar_error(Location::Scope(scope_id), assign.value())
                else {
                    continue;
                };

                let name = self.get_node_str_id(&key, authored.file);
                let element_authored = ElementAuthored {
                    value_node: erase_struct!(value),
                    key_node: Some(erase_struct!(key)),
                };
                if let Err(()) = self.parse_element(
                    ElementKey::Name(name),
                    scope_id,
                    element_authored,
                    authored.file,
                ) {
                    self.diagnose(
                        Location::Scope(scope_id),
                        Diagnostic::ElementKeyRedundancy {
                            source: key.upcast(),
                        },
                    );
                }
            }
        }
        self.publish_scope(scope_id);
        scope_id.in_module
    }
    fn run_value(&mut self, value: Value, element_id: LocalElementId) -> Option<TypedValue> {
        let value = match value {
            Value::Int(x) => TypedValue {
                value: Value::Int(x),
                r#type: Value::IntTy,
            },
            Value::IntTy => TypedValue {
                value: Value::IntTy,
                r#type: Value::TyTy,
            },
            Value::Scope(scope_id) => TypedValue {
                value: Value::Scope(scope_id),
                r#type: Value::ScopeTy,
            },
            Value::ScopeTy => TypedValue {
                value: Value::ScopeTy,
                r#type: Value::TyTy,
            },
            Value::TyTy => TypedValue {
                value: Value::TyTy,
                r#type: Value::TyTy,
            },
            Value::Name { name, scope, node } => {
                if let Some(ref_element_id) = self.find_element_local(scope, name, true) {
                    self.depend_element_value(
                        element_id,
                        ElementId::Local(ref_element_id),
                        node.upcast(),
                    )?
                } else {
                    self.diagnose(
                        Location::Element(element_id),
                        Diagnostic::FailedFindElement {
                            source: node.upcast(),
                        },
                    );
                    return None;
                }
            }
            Value::Find {
                value: ref_element_id,
                key,
                key_source: key_node,
                source: node,
            } => {
                let value = self.depend_child_element_value(element_id, ref_element_id)?;
                match value.value {
                    Value::Scope(scope_id) => {
                        log::error!("find element {}", &*self.id2str(key));
                        if let Some(find_element_id) = self.find_element(scope_id, key, false) {
                            self.depend_element_value(
                                element_id,
                                find_element_id,
                                key_node.upcast(),
                            )?
                        } else {
                            self.diagnose(
                                Location::Element(element_id),
                                Diagnostic::FailedFindElement {
                                    source: key_node.upcast(),
                                },
                            );
                            return None;
                        }
                    }
                    _ => {
                        self.diagnose(
                            Location::Element(element_id),
                            Diagnostic::CanNotFindIn {
                                source: node.upcast(),
                                value: value.value,
                            },
                        );
                        return None;
                    }
                }
            }
            Value::Call {
                func,
                param,
                source,
            } => {
                let func = self.depend_child_element_value(element_id, func)?;
                let param = self.depend_child_element_value(element_id, param)?;
                match func.value {
                    Value::Builtin(builtin) => match builtin {
                        Builtin::If => todo!(),
                        Builtin::Add => todo!(),
                        Builtin::Mod => match param.value {
                            Value::String(string_id) => {
                                let str = erase(self).id2str(string_id);
                                let path = self.get_source_path().join(str.deref());
                                let module_id =
                                    self.depend_module(ModuleAuthored::File { path }, element_id)?;
                                if self.is_module_local(module_id) {
                                    self.run_module(module_id);
                                }
                                let module = self.get_module_remote(module_id);
                                let scope_id = *module.root_scope.get()?;
                                let scope = self.get_scope_remote(RemoteScopeId {
                                    in_module: scope_id,
                                    module: module_id,
                                });
                                TypedValue {
                                    value: Value::Scope(ScopeId {
                                        local: scope.local_id.global(module_id),
                                        remote: Some(scope_id),
                                    }),
                                    r#type: Value::ScopeTy,
                                }
                            }
                            _ => todo!(),
                        },
                    },
                    _ => {
                        self.diagnose(
                            Location::Element(element_id),
                            Diagnostic::CanNotCallOn {
                                source: source.upcast(),
                                value,
                            },
                        );
                        return None;
                    }
                }
            }
            _ => return None,
        };

        Some(value)
    }
    fn set_element_value(&mut self, element_id: LocalElementId, value: TypedValue, resolved: bool);
    fn run_element(&mut self, element_id: LocalElementId) {
        let element = erase_mut(self.get_element_mut(element_id));

        if element.resolved || element.dependency_count > 0 {
            return;
        }

        if let ElementKey::Name(name) = element.key {
            log::error!(
                "run element(: {} = {}",
                &*self.id2str(name),
                ContextedValue {
                    ctx: self,
                    value: &element.raw_value
                }
            );
        } else {
            log::error!(
                "run element(: {:?} = {}",
                element_id.in_module,
                ContextedValue {
                    ctx: self,
                    value: &element.raw_value
                }
            );
        }

        if let Some(resolved_value) = self.run_value(element.raw_value, element_id) {
            self.set_element_value(element_id, resolved_value, element.dependency_count == 0);
        }

        if element.dependency_count > 0 {
            return;
        }

        element.resolved = true;
        if let ElementKey::Name(name) = element.key {
            log::error!(
                "run element): {} = {}",
                &*self.id2str(name),
                ContextedValue {
                    ctx: self,
                    value: &element.resolved_value.value
                }
            );
        } else {
            log::error!(
                "run element): {:?} = {}",
                element_id.in_module,
                ContextedValue {
                    ctx: self,
                    value: &element.resolved_value.value
                }
            );
        }
        for dependant in mem::take(&mut element.dependants) {
            let dependant_element = self.get_element(dependant.element_id);
            if let ElementKey::Name(name) = dependant_element.key {
                log::error!("resolve dependant: {}", &*self.id2str(name),);
            } else {
                log::error!("resolve dependant: {:?}", dependant.element_id.in_module,);
            }
            self.resolve_element(dependant.element_id);
        }
        if let Some(remote_id) = &element.remote_id {
            let remote = self.get_element_remote(RemoteElementId {
                in_module: *remote_id,
                module: element_id.module,
            });
            remote.deref().r#mut.store(ElementRemoteMut {
                value: element.resolved_value,
                resolved: true,
            });
        }
    }
}

pub trait InterpreterLike {
    fn is_module_local(&self, module: ModuleId) -> bool;
    fn get_worksapce_path(&self) -> &Path;
    fn get_source_path(&self) -> &Path;
    fn id2str(&self, id: StringId) -> impl Deref<Target = str>;
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote;
    fn get_module(&self, id: ModuleId) -> &ModuleCell;
    fn get_module_remote(&self, id: ModuleId) -> &ModuleRemote;
    fn get_thread_remote_of(&self, module: ModuleId) -> Option<&ThreadRemote>;
    fn get_element_value(&self, id: ElementId) -> Option<TypedValue>;
    fn find_element(&self, scope: ScopeId, key: StringId, include_super: bool)
    -> Option<ElementId>;
    fn get_element(&self, id: LocalElementId) -> &Element {
        &self.get_module(id.module).elements[id.in_module]
    }
    fn get_scope(&self, id: LocalScopeId) -> &Scope {
        &self.get_module(id.module).scopes[id.in_module]
    }
    fn get_element_remote(&self, id: RemoteElementId) -> impl Deref<Target = ElementRemote> {
        self.get_module_remote(id.module)
            .elements
            .get(id.in_module)
            .unwrap()
    }
    fn get_scope_remote(&self, id: RemoteScopeId) -> impl Deref<Target = ScopeRemote> {
        self.get_module_remote(id.module)
            .scopes
            .get(id.in_module)
            .unwrap()
    }

    fn get_file(&self, id: FileId) -> &File;
    fn get_node_str<'tree>(&self, node: &impl Node<'tree>, file: FileId) -> &str {
        let file = self.get_file(file);
        let start = node.start_byte();
        let end = node.end_byte();
        &file.text[start..end]
    }
    fn find_element_local(
        &self,
        scope_id: LocalScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<LocalElementId> {
        let scope = erase(self.get_scope(scope_id));

        if include_super {
            let mut scope_id_iter = scope_id.in_module;
            let mut scope_iter = scope;
            loop {
                if let Some(element) = scope_iter.elements.get(&key).copied() {
                    return Some(element.global(scope_id.module));
                }
                if let Some(parent_scope) = scope_iter.parent {
                    scope_id_iter = parent_scope;
                    scope_iter = self.get_scope(scope_id_iter.global(scope_id.module));
                } else {
                    return None;
                }
            }
        } else {
            return Some(scope.elements.get(&key)?.global(scope_id.module));
        }
    }
    fn find_element_remote(
        &self,
        scope_id: RemoteScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<RemoteElementId> {
        let scope = self.get_scope_remote(scope_id);

        if include_super {
            let mut scope_id_iter = scope_id.in_module;
            let mut scope_iter = scope;
            loop {
                if let Some(element) = scope_iter.elements.get(&key).copied() {
                    return Some(RemoteElementId {
                        in_module: element,
                        module: scope_id.module,
                    });
                }
                if let Some(parent_scope) = scope_iter.parent {
                    scope_id_iter = parent_scope;
                    scope_iter = self.get_scope_remote(RemoteScopeId {
                        in_module: scope_id_iter,
                        module: scope_id.module,
                    });
                } else {
                    return None;
                }
            }
        } else {
            return Some(RemoteElementId {
                in_module: scope.elements.get(&key).copied()?,
                module: scope_id.module,
            });
        }
    }
}

impl InterpreterLike for Interpreter {
    fn id2str(&self, id: StringId) -> impl Deref<Target = str> {
        self.strings.resolve(id)
    }
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote {
        &self.remote.threads[id].remote
    }
    fn get_thread_remote_of(&self, module: ModuleId) -> Option<&ThreadRemote> {
        if let Some(id) = self.remote.module2thread[module] {
            Some(self.get_thread_remote(id))
        } else {
            None
        }
    }
    fn get_module(&self, id: ModuleId) -> &ModuleCell {
        unsafe { &self.modules[id].cell.as_ref_unchecked() }
    }
    fn get_module_remote(&self, id: ModuleId) -> &ModuleRemote {
        &self.modules[id].remote
    }
    fn get_file(&self, id: FileId) -> &File {
        &self.files[id]
    }
    fn get_element_value(&self, id: ElementId) -> Option<TypedValue> {
        match id {
            ElementId::Local(local_element_id) => {
                let dependency = self.get_element(local_element_id);
                if dependency.resolved {
                    Some(dependency.resolved_value)
                } else {
                    None
                }
            }
            ElementId::Remote(remote_element_id) => unimplemented!(),
        }
    }

    fn find_element(
        &self,
        scope_id: ScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<ElementId> {
        Some(ElementId::Local(self.find_element_local(
            scope_id.local,
            key,
            include_super,
        )?))
    }

    fn get_worksapce_path(&self) -> &Path {
        &self.workspace_path
    }

    fn get_source_path(&self) -> &Path {
        &self.source_path
    }

    fn is_module_local(&self, module: ModuleId) -> bool {
        true
    }
}

impl<IP: Deref<Target = Interpreter>> InterpreterLike for ThreadInterpreter<IP> {
    fn id2str(&self, id: StringId) -> impl Deref<Target = str> {
        self.interpreter.remote.strings.resolve(id)
    }
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote {
        self.interpreter.get_thread_remote(id)
    }
    fn get_thread_remote_of(&self, module: ModuleId) -> Option<&ThreadRemote> {
        self.interpreter.get_thread_remote_of(module)
    }
    fn get_module(&self, id: ModuleId) -> &ModuleCell {
        assert!(!self.is_module_remote(id));
        unsafe { self.interpreter.modules[id].cell.as_ref_unchecked() }
    }
    fn get_module_remote(&self, id: ModuleId) -> &ModuleRemote {
        &self.interpreter.modules[id].remote
    }
    fn get_file(&self, id: FileId) -> &File {
        &self.interpreter.files[id]
    }
    fn get_element_value(&self, dependency_id: ElementId) -> Option<TypedValue> {
        match dependency_id {
            ElementId::Local(local_element_id) => {
                let dependency = self.get_element(local_element_id);
                if dependency.resolved {
                    Some(dependency.resolved_value)
                } else {
                    None
                }
            }
            ElementId::Remote(remote_element_id) => {
                let dependency = self.get_element_remote(remote_element_id);
                let r#mut = dependency.r#mut.load();
                if r#mut.resolved {
                    Some(r#mut.value)
                } else {
                    None
                }
            }
        }
    }

    fn find_element(
        &self,
        scope_id: ScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<ElementId> {
        if self.is_module_local(scope_id.get_module()) {
            Some(ElementId::Local(self.find_element_local(
                scope_id.local,
                key,
                include_super,
            )?))
        } else {
            Some(ElementId::Remote(self.find_element_remote(
                scope_id.get_remote().unwrap(),
                key,
                include_super,
            )?))
        }
    }

    fn get_worksapce_path(&self) -> &Path {
        self.interpreter.get_worksapce_path()
    }

    fn get_source_path(&self) -> &Path {
        self.interpreter.get_source_path()
    }

    fn is_module_local(&self, module: ModuleId) -> bool {
        self.is_module_local(module)
    }
}

impl InterpreterLikeMut for Interpreter {
    fn str2id(&mut self, str: &str) -> StringId {
        self.strings.get_or_intern(str)
    }

    fn get_thread_mut(&mut self, id: ThreadId) -> &mut ThreadMut {
        self.remote.threads[id].r#mut.get_mut()
    }

    fn get_thread_mut_of(&mut self, module: ModuleId) -> Option<&mut ThreadMut> {
        if let Some(id) = self.remote.module2thread[module] {
            Some(self.get_thread_mut(id))
        } else {
            None
        }
    }

    fn get_module_mut(&mut self, id: ModuleId) -> &mut ModuleCell {
        self.modules[id].cell.get_mut()
    }

    fn depend_element(
        &mut self,
        dependant_id: LocalElementId,
        dependency_id: ElementId,
        node: UntypedNode<'static>,
        local: bool,
    ) {
        if local {
            let dependant = erase_mut(self.get_element_mut(dependant_id));
            dependant.dependency_count += 1;
        }
        match dependency_id {
            ElementId::Local(local_element_id) => {
                let dependency = erase_mut(self.get_element_mut(local_element_id));
                dependency.dependants.push(Dependant {
                    element_id: dependant_id,
                    node,
                });
            }
            _ => unimplemented!(),
        }
    }

    fn resolve_element(&mut self, id: LocalElementId) {
        let dependant = self.get_element_mut(id);
        dependant.dependency_count -= 1;
        self.run_element(id)
    }

    fn set_element_value(&mut self, element_id: LocalElementId, value: TypedValue, resolved: bool) {
        let element = self.get_element_mut(element_id);
        element.resolved_value = value;
        element.resolved = resolved;
    }

    fn depend_module_raw(
        &mut self,
        authored: ModuleAuthored,
        element_id: LocalElementId,
    ) -> Option<ModuleId> {
        Some(self.add_module(authored))
    }
}

impl<IP: Deref<Target = Interpreter>> InterpreterLikeMut for ThreadInterpreter<IP> {
    fn str2id(&mut self, str: &str) -> StringId {
        self.interpreter.remote.strings.get_or_intern(str)
    }

    fn get_thread_mut(&mut self, id: ThreadId) -> &mut ThreadMut {
        assert!(id == self.thread);
        unsafe { self.interpreter.remote.threads[id].r#mut.as_mut_unchecked() }
    }

    fn get_thread_mut_of(&mut self, module: ModuleId) -> Option<&mut ThreadMut> {
        if let Some(id) = self.interpreter.remote.module2thread[module] {
            Some(self.get_thread_mut(id))
        } else {
            None
        }
    }

    fn get_module_mut(&mut self, id: ModuleId) -> &mut ModuleCell {
        assert!(self.is_module_local(id));
        unsafe { self.interpreter.modules[id].cell.as_mut_unchecked() }
    }

    fn depend_element(
        &mut self,
        dependant_id: LocalElementId,
        dependency_id: ElementId,
        node: UntypedNode<'static>,
        local: bool,
    ) {
        if local {
            let dependant = self.get_element_mut(dependant_id);
            dependant.dependency_count += 1;
            if let ElementKey::Name(name) = dependant.key {
                log::error!("depend_element: dependant: {}", &*self.id2str(name));
            } else {
                log::error!("depend_element: dependant: {:?}", dependant_id.in_module);
            }
        }

        match dependency_id {
            ElementId::Local(local_element_id) => {
                if self.is_module_local(local_element_id.module) {
                    let dependency = erase_mut(self.get_element_mut(local_element_id));
                    if let ElementKey::Name(name) = dependency.key {
                        log::error!("depend_element: dependency: {}", &*self.id2str(name));
                    } else {
                        log::error!(
                            "depend_element: dependency: {:?}",
                            local_element_id.in_module
                        );
                    }
                    dependency.dependants.push(Dependant {
                        element_id: dependant_id,
                        node,
                    });
                } else {
                    if let Some(thread) = self.get_thread_remote_of(local_element_id.module) {
                        thread.channel.push(Signal::Depend(Depend {
                            dependant: dependant_id,
                            dependency: local_element_id,
                            node,
                        }));
                    }
                }
            }
            ElementId::Remote(remote_element_id) => {
                let dependency = self.get_element_remote(remote_element_id);
                if let Some(thread) = self.get_thread_remote_of(remote_element_id.module) {
                    thread.channel.push(Signal::Depend(Depend {
                        dependant: dependant_id,
                        dependency: dependency.deref().local_id.global(remote_element_id.module),
                        node,
                    }));
                }
            }
        }
    }

    fn resolve_element(&mut self, id: LocalElementId) {
        let thread = self.interpreter.remote.module2thread[id.module].unwrap();

        if thread == self.thread {
            let dependant = self.get_element_mut(id);
            dependant.dependency_count -= 1;
            self.run_element(id)
        } else {
            let thread = self.get_thread_remote(thread);
            thread.channel.push(Signal::Resolve(id));
        }
    }

    fn set_element_value(&mut self, element_id: LocalElementId, value: TypedValue, resolved: bool) {
        let element = self.get_element_mut(element_id);
        element.resolved_value = value;
        element.resolved = resolved;
        if let Some(remote_id) = element.remote_id.as_ref().copied() {
            let element = self.get_element_remote(RemoteElementId {
                in_module: remote_id,
                module: element_id.module,
            });
            element.r#mut.store(ElementRemoteMut {
                value,
                resolved: resolved,
            });
        }
    }

    fn depend_module_raw(
        &mut self,
        authored: ModuleAuthored,
        element_id: LocalElementId,
    ) -> Option<ModuleId> {
        let element = self.get_element_mut(element_id);
        element.dependency_count += 1;
        let add_module_delay = &mut self.get_thread_mut(self.thread).add_module_delay;
        match authored {
            ModuleAuthored::File { path } => {
                add_module_delay
                    .files
                    .entry(path)
                    .or_default()
                    .push(element_id);
            }
            ModuleAuthored::Scope { file, source } => {
                add_module_delay.scopes.push(AddModuleDelayScope {
                    file,
                    scope: source,
                    element: element_id,
                });
            }
        }
        None
    }
}
