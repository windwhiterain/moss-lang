use crate::erase_struct;
use crate::interpreter::diagnose::Diagnostic;
use crate::interpreter::element::Dependant;
use crate::interpreter::element::Element;
use crate::interpreter::element::ElementAuthored;
use crate::interpreter::element::ElementDescriptor;
use crate::interpreter::element::ElementKey;
use crate::interpreter::element::ElementLocal;
use crate::interpreter::element::ElementSource;
use crate::interpreter::file::File;
use crate::interpreter::file::FileId;
use crate::interpreter::function::Function;
use crate::interpreter::function::FunctionElement;
use crate::interpreter::function::FunctionElementAuthored;
use crate::interpreter::function::FunctionOptimized;
use crate::interpreter::function::FunctionScope;
use crate::interpreter::function::IN_OPTIMIZED;
use crate::interpreter::module::Module;
use crate::interpreter::module::ModuleId;
use crate::interpreter::module::ModuleLocal;
use crate::interpreter::scope::Scope;
use crate::interpreter::scope::ScopeAuthored;
use crate::interpreter::scope::ScopeLocal;
use crate::interpreter::scope::ScopeSource;
use crate::interpreter::thread::Depend;
use crate::interpreter::thread::Signal;
use crate::interpreter::thread::Thread;
use crate::interpreter::thread::ThreadId;
use crate::interpreter::thread::ThreadLocal;
use crate::interpreter::thread::ThreadRemote;
use crate::interpreter::value::Builtin;
use crate::interpreter::value::Expr;
use crate::interpreter::value::StaticValue;
use crate::interpreter::value::Type;
use crate::interpreter::value::Value;
use crate::merge_in;
use crate::utils::concurrent_string_interner::ConcurentInterner;
use crate::utils::concurrent_string_interner::StringId;
use crate::utils::erase;
use crate::utils::erase_mut;
use crate::utils::secondary_linked_list::List;
use crate::utils::unsafe_cell::UnsafeCell;
use slotmap::SecondaryMap;
use slotmap::SlotMap;
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::fmt::Display;
use std::hash::Hash;
use std::iter;
use std::marker::PhantomData;
use std::mem;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread::available_parallelism;
use tokio::sync::Notify;
use tokio::sync::futures::Notified;
use tokio::task::JoinSet;

use crate::utils::moss;
use type_sitter::HasChild;
pub use type_sitter::Node;
use type_sitter::NodeResult;
pub use type_sitter::UntypedNode;
pub type Tree = type_sitter::Tree<moss::SourceFile<'static>>;
use crate::utils::type_key::Vec as KeyVec;

pub mod diagnose;
pub mod element;
pub mod file;
pub mod function;
pub mod module;
pub mod scope;
pub mod thread;
pub mod value;

static SRC_FILE_EXTENSION: &str = "moss";
static SRC_PATH: &str = "src";

pub struct Id<T>(pub usize, PhantomData<T>);

impl<T> Id<T> {
    pub const fn from_idx(idx: usize) -> Self {
        Self(idx, Default::default())
    }
    pub fn from_ptr(ptr: *const T) -> Self {
        Self(ptr as usize, Default::default())
    }
}

impl<T> Id<T> {
    fn with_ctx<'a, Ctx: InterpreterLike + ?Sized>(self, ctx: &'a Ctx) -> ContextedId<'a, T, Ctx> {
        ContextedId { id: self, ctx }
    }
}

pub enum Owner<T> {
    Module(ModuleId),
    Managed(Id<T>),
}

pub trait Managed {
    type Local;
    type Onwer: Managed;
    const NAME: &str;
    fn get_id(&self) -> Id<Self>
    where
        Self: Sized,
    {
        Id::from_ptr((self as *const Self) as *mut Self)
    }
    fn get_local(&self) -> &UnsafeCell<Self::Local>;
    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local>;
    fn get_owner(&self) -> Owner<Self::Onwer>
    where
        Self: Sized;
}

impl<T> Debug for Id<T>
where
    T: Managed,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Id({:?})", T::NAME, self.0)
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

pub struct ContextedId<'a, T, Ctx: ?Sized> {
    id: Id<T>,
    ctx: &'a Ctx,
}

impl<'a, Ctx: InterpreterLike + ?Sized> Display for ContextedId<'a, Element, Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if let Some(value) = self.ctx.get_element_value(self.id) {
                value
            } else {
                Value::Static(StaticValue::Err)
            }
            .with_ctx(self.ctx)
        )
    }
}

impl<'a, Ctx: InterpreterLike + ?Sized> Display for ContextedId<'a, Scope, Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scope = self.ctx.get(self.id);
        write!(f, "{{")?;
        for (key, element) in &scope.elements {
            write!(
                f,
                "{} = {}; ",
                self.ctx.id2str(*key).deref(),
                element.with_ctx(self.ctx)
            )?;
        }
        write!(f, "}}")
    }
}

#[macro_export]
macro_rules! in_module_id {
    ($in_module:ident,$global:ident) => {
        impl InModuleId for $in_module {
            type GlobalId = $global;

            fn global(self, module: ModuleId) -> Self::GlobalId {
                Self::GlobalId {
                    in_module: self,
                    module,
                }
            }
        }
    };
}

pub struct Interpreter {
    pub workspace_path: PathBuf,
    pub strings: StringInterner,
    pub files: SlotMap<FileId, File>,
    pub path2file: hashbrown::HashMap<PathBuf, FileId>,
    pub modules: SlotMap<ModuleId, Module>,
    pub unresolved_modules: List<ModuleId>,
    pub concurrent: InterpreterConcurrent,
    pub is_concurrent: bool,
    pub builtin_module: Option<ModuleId>,
}

unsafe impl Sync for Interpreter {}

impl Interpreter {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            workspace_path: workspace_path,
            strings: StringInterner::new(),
            files: Default::default(),
            path2file: Default::default(),
            modules: Default::default(),
            unresolved_modules: Default::default(),
            concurrent: InterpreterConcurrent {
                module2thread: Default::default(),
                threads: Default::default(),
                strings: ConcurentInterner::new(),
                workload: AtomicUsize::new(0),
                workload_zero: Notify::new(),
            },
            is_concurrent: false,
            builtin_module: Default::default(),
        }
    }
    pub fn init(&mut self) {
        let module = self.add_module(None);
        let scope = erase_mut(unsafe { self.add_scope(None, None, module) });
        let mod_id = self.str2id("mod");
        self.add_element(
            ElementKey::Name(mod_id),
            scope,
            ElementAuthored::Value(Value::Static(StaticValue::Builtin(Builtin::Mod))),
        );
        let diagnose_id = self.str2id("diagnose");
        self.add_element(
            ElementKey::Name(diagnose_id),
            scope,
            ElementAuthored::Value(Value::Static(StaticValue::Builtin(Builtin::Diagnose))),
        );
        self.set_root_scope(module, scope.get_id());
        self.builtin_module = Some(module);
    }
    pub fn clear(&mut self) {
        self.builtin_module = Default::default();
        for file in self.files.values_mut() {
            file.is_module = None;
        }
        self.modules.clear();
        self.unresolved_modules.clear();
        self.concurrent.module2thread.clear();
        self.concurrent.threads.clear();
    }
    pub fn get_file_mut(&mut self, id: FileId) -> &mut File {
        &mut self.files[id]
    }
    pub fn find_file(&self, path: impl AsRef<Path>) -> Option<FileId> {
        self.path2file.get(path.as_ref()).copied()
    }
    pub fn find_or_add_file(&mut self, path: Cow<PathBuf>) -> FileId {
        match erase_mut(self)
            .path2file
            .raw_entry_mut()
            .from_key(path.as_path())
        {
            hashbrown::hash_map::RawEntryMut::Occupied(raw_occupied_entry_mut) => {
                *raw_occupied_entry_mut.get()
            }
            hashbrown::hash_map::RawEntryMut::Vacant(raw_vacant_entry_mut) => {
                let path = path.into_owned();
                let file = File::new(path.clone(), self);
                let file_id = self.files.insert(file);
                raw_vacant_entry_mut.insert(path, file_id);
                file_id
            }
        }
    }
    pub fn add_module(&mut self, path: Option<PathBuf>) -> ModuleId {
        let resolved = path.is_none();
        let authored = if let Some(path) = path {
            let file_id = self.find_or_add_file(Cow::Owned(path));
            let file = erase_mut(self).get_file(file_id);
            Some(ScopeAuthored {
                source: ScopeSource::File(file.tree.root_node().unwrap()),
                file: file_id,
            })
        } else {
            None
        };

        let id = self.modules.insert(Module::new(authored, resolved));
        if let Some(authored) = authored {
            self.get_file_mut(authored.file).is_module = Some(id);
            self.unresolved_modules.push(id);
            self.increase_workload();
        }
        id
    }
    pub async fn run(&mut self) {
        log::error!("run(");
        assert!(!self.is_concurrent);
        loop {
            self.concurrent.module2thread.clear();
            self.concurrent.threads.clear();
            self.concurrent.strings.sync_from(&self.strings);
            let mut thread_num: usize = available_parallelism().unwrap().into();
            let mut module_num = self.unresolved_modules.len();
            if module_num == 0 {
                break;
            }
            log::error!(
                "run loop(: thread_num: {}, module_num: {}",
                thread_num,
                module_num
            );
            let mut modules = self.unresolved_modules.iter();
            loop {
                let mut module_per_thread = module_num.div_ceil(thread_num);
                if module_per_thread == 0 {
                    break;
                }
                let mut module_ids = vec![];
                loop {
                    let id = modules.next().unwrap();
                    module_ids.push(id);
                    let thread_id = self
                        .concurrent
                        .threads
                        .insert(Thread::new(Default::default()));
                    self.concurrent.module2thread.insert(id, thread_id);
                    module_per_thread -= 1;
                    module_num -= 1;
                    if module_per_thread == 0 {
                        let thread = self.concurrent.threads.get_mut(thread_id);
                        thread.local.get_mut().modules = module_ids;
                        break;
                    }
                }
                thread_num -= 1;
            }
            self.is_concurrent = true;
            let mut set = JoinSet::new();
            for thread in self.concurrent.threads.keys() {
                let mut thread_interpreter = ThreadedInterpreter {
                    interpreter: erase(self),
                    thread,
                    workload_zero: Some(erase(self).concurrent.workload_zero.notified()),
                };
                set.spawn(async move { thread_interpreter.run().await });
            }
            log::error!("join_all(");
            set.join_all().await;
            log::error!("join_all)");
            self.strings.sync_from(&self.concurrent.strings);
            self.is_concurrent = false;
            erase_mut(self)
                .unresolved_modules
                .retain(|key| unsafe { self.get_module_local(key) }.is_resolved());
            let mut has_new_module = false;
            for thread_id in erase(self).concurrent.threads.keys() {
                let thread = erase_mut(self).get_thread_local_mut(thread_id);
                for (path, dependants) in mem::take(&mut thread.add_module_delay.files) {
                    let module_id = self.add_module(Some(path));
                    let module = unsafe { self.get_module_local_mut(module_id) };
                    for dependant in dependants.iter().copied() {
                        module.dependants.push(dependant);
                    }
                    has_new_module = true;
                }
            }
            log::error!("run loop)");
            if !has_new_module {
                break;
            }
        }
        log::error!("run)");
        log::error!("interpreter: {:#?}", self.modules);
    }
}

pub struct InterpreterConcurrent {
    pub module2thread: SecondaryMap<ModuleId, ThreadId>,
    pub threads: KeyVec<ThreadId, Thread>,
    pub strings: ConcurrentStringInterner,
    pub workload: AtomicUsize,
    pub workload_zero: Notify,
}

pub struct ThreadedInterpreter<'a, IP: Deref<Target = Interpreter>> {
    pub interpreter: IP,
    pub thread: ThreadId,
    pub workload_zero: Option<Notified<'a>>,
}

impl<'a, IP: Deref<Target = Interpreter>> ThreadedInterpreter<'a, IP> {
    async fn run(&mut self) {
        let modules = &mut erase_mut(self).get_thread_local_mut(self.thread).modules;
        log::error!(
            "run thread(: {:?}, modules_num: {}",
            self.thread,
            modules.len()
        );
        let mut unresolved_num = modules.len();
        for module_id in modules.iter().copied() {
            let module = erase_mut(unsafe { self.get_module_local_mut(module_id) });
            if !module.has_runed() {
                unsafe { self.run_module(module_id) };
            }
            if module.is_resolved() {
                unresolved_num -= 1;
            }
        }
        let thread = erase(self.get_thread_remote(self.thread));
        let terminate = mem::take(&mut self.workload_zero).unwrap();
        let mut run_signal = async || {
            loop {
                if unresolved_num == 0 {
                    log::error!("run thread): unresolved_num, {:?}", self.thread);
                    break;
                }
                if let Some(signal) = thread.channel.pop() {
                    match signal {
                        Signal::Depend(depend) => {
                            self.depend_element(
                                depend.dependant,
                                depend.dependency,
                                depend.source,
                                false,
                            );
                        }
                        Signal::Resolve(local_element_id) => {
                            self.resolve_element(local_element_id);
                        }
                    }
                    self.decrease_workload();
                } else {
                    match thread.channel.async_pop().await {
                        Signal::Depend(depend) => {
                            unsafe {
                                self.get_local_mut(depend.dependency)
                                    .dependants
                                    .push(Dependant {
                                        element_id: depend.dependant,
                                        source: depend.source,
                                    })
                            };
                        }
                        Signal::Resolve(local_element_id) => {
                            self.resolve_element(local_element_id);
                        }
                    }
                    log::error!("async_pop: thread: {:?}", self.thread);
                    self.decrease_workload();
                }
            }
        };
        tokio::select! {
            _ = run_signal()=>{},
            _=terminate=>{
                log::error!("run thread): workload_zero, {:?}", self.thread);
            }
        }
    }
}

pub type StringInterner = crate::utils::concurrent_string_interner::Interner;

pub type ConcurrentStringInterner = crate::utils::concurrent_string_interner::ConcurentInterner;

pub enum Location {
    Element(Id<Element>),
    Scope(Id<Scope>),
}
pub trait InterpreterLike {
    fn is_concurrent(&self) -> bool;
    fn is_local_module(&self, id: ModuleId) -> bool;
    fn is_remote_module(&self, id: ModuleId) -> bool;
    fn get_module_of<T: Managed>(&self, id: Id<T>) -> ModuleId {
        match self.get::<T>(id).get_owner() {
            Owner::Module(module_id) => module_id,
            Owner::Managed(id) => self.get_module_of(id),
        }
    }
    fn is_local<T: Managed>(&self, id: Id<T>) -> bool {
        self.is_local_module(self.get_module_of(id))
    }

    fn is_remote<T: Managed>(&self, id: Id<T>) -> bool {
        self.is_remote_module(self.get_module_of(id))
    }
    fn get_worksapce_path(&self) -> &Path;
    /// # Panic
    /// run without init since new or last clear.
    fn get_builtin_module(&self) -> ModuleId;
    fn get_file(&self, id: FileId) -> &File;
    fn find_file(&self, path: impl AsRef<Path>) -> Option<FileId>;
    fn id2str(&self, id: StringId) -> impl Deref<Target = str>;
    fn get_source_str<'tree>(&self, source: &impl Node<'tree>, file: FileId) -> &str {
        let file = self.get_file(file);
        let start = source.start_byte();
        let end = source.end_byte();
        &file.text[start..end]
    }
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote;
    /// # Returns
    /// - `None` if when concurrent, module is not in threads.
    fn get_thread_remote_of_module(&self, module: ModuleId) -> Option<&ThreadRemote>;
    fn get_thread_remote_of<T: Managed>(&self, id: Id<T>) -> Option<&ThreadRemote> {
        self.get_thread_remote_of_module(self.get_module_of(id))
    }
    fn get_module(&self, id: ModuleId) -> &Module;
    /// # Safety
    /// - `id` is not remote
    unsafe fn get_module_local(&self, id: ModuleId) -> &ModuleLocal {
        debug_assert!(!self.is_remote_module(id));
        unsafe { self.get_module(id).local.as_ref_unchecked() }
    }

    fn get<T>(&self, id: Id<T>) -> &T {
        unsafe { &*(id.0 as *const T) }
    }
    /// # Safety
    /// `id` is not remote.
    unsafe fn get_local<'a, T: Managed + 'a>(&'a self, id: Id<T>) -> &'a T::Local {
        debug_assert!(!self.is_remote(id));
        unsafe { self.get::<T>(id).get_local().as_ref_unchecked() }
    }

    fn get_element_value(&self, id: Id<Element>) -> Option<Value> {
        if self.is_remote(id) {
            self.get(id).value.get().copied()
        } else {
            let local = unsafe { self.get_local(id) };
            local.value
        }
    }
    fn find_element(
        &self,
        scope_id: Id<Scope>,
        key: StringId,
        include_super: bool,
    ) -> Option<Id<Element>> {
        if let Some(raw) = {
            let scope = self.get::<Scope>(scope_id);
            if include_super {
                let mut scope_iter = scope;
                loop {
                    if let Some(element) = scope_iter.elements.get(&key).copied() {
                        break Some(element);
                    }
                    if let Some(parent_scope) = scope_iter.parent {
                        scope_iter = self.get::<Scope>(parent_scope);
                    } else {
                        break None;
                    }
                }
            } else {
                scope.elements.get(&key).copied()
            }
        } {
            Some(raw)
        } else {
            let builtin_module = self.get_builtin_module();
            let scope = self
                .get::<Scope>(unsafe { self.get_module_local(builtin_module).root_scope.unwrap() });
            if let Some(id) = scope.elements.get(&key).copied() {
                Some(id)
            } else {
                None
            }
        }
    }
    fn collect<Ctx>(
        &self,
        ctx: &mut Ctx,
        collect: impl FnMut(&mut Ctx, Id<Element>, Option<Expr>) -> Id<Element>,
        if_terminate: impl Fn(&Ctx, Id<Element>) -> bool,
        map: impl Fn(&Ctx, Id<Element>) -> Option<Id<Element>>,
        start: Id<Element>,
    ) -> Id<Element> {
        struct Context<'a, Ctx, IP: ?Sized, Collect, IfTermintate, Map> {
            ctx: &'a mut Ctx,
            interpreter: &'a IP,
            map: Map,
            collect: Collect,
            if_terminate: IfTermintate,
        }
        let mut context = Context {
            ctx,
            interpreter: self,
            map,
            collect,
            if_terminate,
        };
        impl<
            'a,
            Ctx,
            IP: InterpreterLike + ?Sized,
            Collect: FnMut(&mut Ctx, Id<Element>, Option<Expr>) -> Id<Element>,
            IfTermintate: Fn(&Ctx, Id<Element>) -> bool,
            Map: Fn(&Ctx, Id<Element>) -> Option<Id<Element>>,
        > Context<'a, Ctx, IP, Collect, IfTermintate, Map>
        {
            fn traverse(&mut self, element_id: Id<Element>) -> Id<Element> {
                if let Some(id) = (self.map)(self.ctx, element_id) {
                    return id;
                }

                let expr = if !(self.if_terminate)(self.ctx, element_id)
                    && let Some(expr) = unsafe { self.interpreter.get_local(element_id) }.expr
                {
                    Some(expr.map_ref(|x| self.traverse(x)))
                } else {
                    None
                };
                (self.collect)(self.ctx, element_id, expr)
            }
        }
        context.traverse(start)
    }
    fn traverse<Ctx>(
        &self,
        ctx: &mut Ctx,
        visit: impl FnMut(&mut Ctx, Id<Element>),
        if_terminate: impl Fn(&Ctx, Id<Element>) -> bool,
        visited: impl Fn(&Ctx, Id<Element>) -> bool,
        start: Id<Element>,
    ) {
        struct Context<'a, Ctx, IP: ?Sized, Visit, IfTermintate, Visited> {
            ctx: &'a mut Ctx,
            interpreter: &'a IP,
            visited: Visited,
            visit: Visit,
            if_terminate: IfTermintate,
        }
        let mut context = Context {
            ctx,
            interpreter: self,
            visited,
            visit,
            if_terminate,
        };
        impl<
            'a,
            Ctx,
            IP: InterpreterLike + ?Sized,
            Visit: FnMut(&mut Ctx, Id<Element>),
            IfTermintate: Fn(&Ctx, Id<Element>) -> bool,
            Visited: Fn(&Ctx, Id<Element>) -> bool,
        > Context<'a, Ctx, IP, Visit, IfTermintate, Visited>
        {
            fn traverse(&mut self, element_id: Id<Element>) {
                if (self.visited)(self.ctx, element_id) {
                    return;
                }

                if !(self.if_terminate)(self.ctx, element_id)
                    && let Some(expr) = unsafe { self.interpreter.get_local(element_id) }.expr
                {
                    expr.iter_ref(|x| self.traverse(x));
                }
                (self.visit)(self.ctx, element_id)
            }
        }
        context.traverse(start)
    }
}

pub trait InterpreterLikeMut: InterpreterLike {
    fn increase_workload(&mut self);
    fn decrease_workload(&mut self) -> usize;
    fn str2id(&mut self, str: &str) -> StringId;
    fn get_source_str_id<'tree>(&mut self, source: &impl Node<'tree>, file: FileId) -> StringId {
        let str = erase(self.get_source_str(source, file));
        self.str2id(str)
    }
    /// # Panic
    /// when concurrent, thread is not local.
    fn get_thread_local_mut(&mut self, id: ThreadId) -> &mut ThreadLocal;
    /// # Returns
    /// - `None` if module is not in threads.
    /// # Panic
    /// when concurrent, module is not in local thread.
    fn get_thread_local_mut_of(&mut self, module: ModuleId) -> Option<&mut ThreadLocal>;
    unsafe fn get_module_mut_raw(&mut self, id: ModuleId) -> &mut Module;
    /// # Safety
    /// - `id` is local.
    unsafe fn get_module_local_mut(&mut self, id: ModuleId) -> &mut ModuleLocal {
        debug_assert!(self.is_local_module(id));
        unsafe { self.get_module(id).local.as_mut_unchecked() }
    }
    /// # Safety
    /// - `self` is not concurrent.
    unsafe fn get_module_mut(&mut self, id: ModuleId) -> &mut Module {
        debug_assert!(!self.is_concurrent());
        unsafe { self.get_module_mut_raw(id) }
    }
    /// # Safety
    /// `id` must in local thread.
    unsafe fn get_local_mut<'a, T: Managed + 'a>(&'a mut self, id: Id<T>) -> &'a mut T::Local {
        debug_assert!(self.is_local(id));
        unsafe {
            let value = self.get::<T>(id);
            value.get_local().as_mut_unchecked()
        }
    }
    /// # Safety
    /// - not concurrent.
    unsafe fn get_mut<T>(&mut self, id: Id<T>) -> &mut T {
        debug_assert!(!self.is_concurrent());
        unsafe { &mut *(id.0 as *mut T) }
    }
    /// # Safety
    /// - parent must be in local thread.
    /// - module is not in local thread.
    unsafe fn add_scope(
        &mut self,
        parent: Option<Id<Scope>>,
        authored: Option<ScopeAuthored>,
        module: ModuleId,
    ) -> &mut Scope {
        let depth = if let Some(parent) = parent {
            self.get(parent).depth + 1
        } else {
            0
        };
        let scope = unsafe {
            self.get_module_local_mut(module)
                .pools
                .get_mut::<Scope>()
                .insert(Scope::new(parent, authored, module, depth))
        };
        let scope_id = scope.get_id();
        let scope = erase_mut(scope);
        if let Some(parent) = parent {
            let parent = unsafe { self.get_local_mut::<Scope>(parent) };
            parent.children.push(scope_id);
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
                log::error!("assign: {assign:?}");
                let Some(assign) =
                    (unsafe { self.grammar_error(Location::Scope(scope_id), assign) })
                else {
                    continue;
                };

                let Some(key) =
                    (unsafe { self.grammar_error(Location::Scope(scope_id), assign.key()) })
                else {
                    continue;
                };
                let Some(value) =
                    (unsafe { self.grammar_error(Location::Scope(scope_id), assign.value()) })
                else {
                    continue;
                };

                let name = self.get_source_str_id(&key, authored.file);
                let element_authored = ElementAuthored::Source {
                    source: ElementSource {
                        value_source: erase_struct!(value),
                        key_source: Some(erase_struct!(key)),
                    },
                    file: authored.file,
                };
                let _ = self.add_element(ElementKey::Name(name), scope, element_authored);
            }
        }
        scope
    }
    fn add_element(
        &mut self,
        key: ElementKey,
        scope: &mut Scope,
        authored: ElementAuthored,
    ) -> Result<Id<Element>, ()> {
        struct Context<'a, IP: ?Sized> {
            interpreter: &'a mut IP,
            key: ElementKey,
            scope: &'a mut Scope,
            authored: ElementAuthored,
        }
        let mut ctx = Context {
            interpreter: self,
            key,
            scope,
            authored
        };

        impl<'a, IP: InterpreterLikeMut + ?Sized> Context<'a, IP> {
            fn add_raw(&mut self) -> &mut Element {
                let value = unsafe { self.interpreter.get_module_local_mut(self.scope.module) }
                    .pools
                    .get_mut::<Element>()
                    .insert(Element::new(self.key, self.scope.get_id()));
                value
            }
            fn add(&mut self)->Option<&mut Element> {
                match self.key {
                    ElementKey::Name(name) => match erase_mut(self).scope.elements.entry(name) {
                        std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                            if let ElementAuthored::Source { source, file } = self.authored {
                                let source = if let Some(key_source) = source.key_source {
                                    key_source.upcast()
                                } else {
                                    source.value_source.upcast()
                                };
                                unsafe {
                                    self.interpreter.diagnose(
                                        Location::Scope(self.scope.get_id()),
                                        Diagnostic::RedundantElementKey { source },
                                    )
                                };
                            }
                            return None;
                        }
                        std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                            let id = self.add_raw();
                            vacant_entry.insert(id.get_id());
                            Some(id)
                        }
                    },
                    ElementKey::Temp => Some(self.add_raw()),
                }
            }
        }
        let element = erase_mut(ctx.add().ok_or(())?);
        match authored {
            ElementAuthored::Source { source, file } => {
                element.source = Some(source);
                let expr = unsafe {
                    self.parse_value(Ok(source.value_source), element.get_id(), scope, file)
                };
                let element_local = element.local.get_mut();
                element_local.expr = expr;

                let module = unsafe { self.get_module_local_mut(scope.module) };
                module.unresolved_count += 1;
                let unresolved_count = module.unresolved_count;
                log::error!("{:?}: add {}", element.get_id(), unresolved_count);
            }
            ElementAuthored::Value(value) => {
                let element_local = element.local.get_mut();
                element_local.value = Some(value);
                element.value = OnceLock::from(value);
            }
            ElementAuthored::Expr(expr) => {
                let element_local = element.local.get_mut();
                element_local.expr = Some(expr);

                let module = unsafe { self.get_module_local_mut(scope.module) };
                module.unresolved_count += 1;
                let unresolved_count = module.unresolved_count;
                log::error!("{:?}: add {}", element.get_id(), unresolved_count);
            }
        }
        Ok(element.get_id())
    }
    /// # Safety
    /// - `element_id` is local.
    /// - `element_id` is in scope `parent`.
    unsafe fn parse_value(
        &mut self,
        source: NodeResult<'static, moss::Value<'static>>,
        element_id: Id<Element>,
        parent: &mut Scope,
        file_id: FileId,
    ) -> Option<Expr> {
        let source = unsafe { self.grammar_error(Location::Element(element_id), source) }?;
        let source_child =
            unsafe { self.grammar_error(Location::Element(element_id), source.child()) }?;
        let element = self.get::<Element>(element_id);
        let scope_id = element.scope;
        debug_assert!(scope_id == parent.get_id());
        let expr = match source_child {
            moss::ValueChild::Bracket(bracket) => unsafe {
                self.parse_value(bracket.value(), element_id, parent, file_id)?
            },
            moss::ValueChild::Call(call) => {
                let func =
                    unsafe { self.grammar_error(Location::Element(element_id), call.func()) }?;
                let param =
                    unsafe { self.grammar_error(Location::Element(element_id), call.param()) }?;
                let func_element = self
                    .add_element(
                        ElementKey::Temp,
                        parent,
                        ElementAuthored::Source {
                            source: ElementSource {
                                value_source: func,
                                key_source: None,
                            },
                            file: file_id,
                        },
                    )
                    .unwrap();
                let param_element = self
                    .add_element(
                        ElementKey::Temp,
                        parent,
                        ElementAuthored::Source {
                            source: ElementSource {
                                value_source: param,
                                key_source: None,
                            },
                            file: file_id,
                        },
                    )
                    .unwrap();
                Expr::Call {
                    func: func_element,
                    param: param_element,
                    source: call,
                }
            }
            moss::ValueChild::Scope(scope_source) => Expr::Value(StaticValue::Scope(unsafe {
                // SAFETY: element -> scope
                self.add_scope(
                    Some(scope_id),
                    Some(ScopeAuthored {
                        source: ScopeSource::Scope(scope_source),
                        file: file_id,
                    }),
                    parent.module,
                )
                .get_id()
            })),
            moss::ValueChild::Find(find) => {
                let value =
                    unsafe { self.grammar_error(Location::Element(element_id), find.value()) }?;
                let name =
                    unsafe { self.grammar_error(Location::Element(element_id), find.name()) }?;
                let element = self
                    .add_element(
                        ElementKey::Temp,
                        parent,
                        ElementAuthored::Source {
                            source: ElementSource {
                                value_source: value,
                                key_source: None,
                            },
                            file: file_id,
                        },
                    )
                    .unwrap();
                Expr::FindIn {
                    value: element,
                    key: self.get_source_str_id(&name, file_id),
                    key_source: name,
                    source: find,
                }
            }
            moss::ValueChild::Int(int) => Expr::Value(StaticValue::Int(
                self.get_source_str(&int, file_id).parse().unwrap(),
            )),
            moss::ValueChild::Name(name) => {
                let string_id = self.get_source_str_id(&name, file_id);
                Expr::Find {
                    name: string_id,
                    source: name,
                }
            }
            moss::ValueChild::String(string) => {
                let mut cursor = erase_struct!(self.get_file(file_id).tree.walk());
                let mut value: Option<Cow<str>> = None;
                for content in string.contents(erase_mut(&mut cursor)) {
                    let content =
                        erase_mut(self).grammar_error(Location::Element(element_id), content)?;
                    let content_value = match erase_mut(self)
                        .grammar_error(Location::Element(element_id), content.child())?
                    {
                        moss::StringContentChild::StringEscape(string_escape) => {
                            match erase(self).get_source_str(&string_escape, file_id) {
                                "\\\"" => Some("\""),
                                "\\\\" => Some("\\"),
                                "\\n" => Some("\n"),
                                "\\t" => Some("\t"),
                                "\\r" => Some("\r"),
                                "\\{" => Some("{"),
                                "\\}" => Some("}"),
                                _ => {
                                    unsafe {
                                        erase_mut(self).diagnose(
                                            Location::Element(element_id),
                                            Diagnostic::StringEscapeError {
                                                source: string_escape.upcast(),
                                            },
                                        )
                                    };
                                    None
                                }
                            }
                        }
                        moss::StringContentChild::StringRaw(string_raw) => {
                            Some(erase(self).get_source_str(&string_raw, file_id))
                        }
                    }?;
                    if let Some(value) = &mut value {
                        value.to_mut().push_str(content_value);
                    } else {
                        value = Some(Cow::Borrowed(content_value))
                    }
                }

                Expr::Value(StaticValue::String(
                    self.str2id(value.as_ref().map(|x| x.as_ref()).unwrap_or("")),
                ))
            }
            moss::ValueChild::Meta(meta) => {
                let name = self.grammar_error(Location::Element(element_id), meta.name())?;
                let string_id = self.get_source_str_id(&name, file_id);
                Expr::MetaFind {
                    name: string_id,
                    source: meta,
                }
            }
            moss::ValueChild::Function(function) => {
                let r#in = self.grammar_error(Location::Element(element_id), function.in_())?;
                let r#in = self.get_source_str_id(&r#in, file_id);
                let scope = self.grammar_error(Location::Element(element_id), function.scope())?;
                let scope = unsafe {
                    // SAFETY: element -> scope
                    erase_mut(self).add_scope(
                        Some(scope_id),
                        Some(ScopeAuthored {
                            source: ScopeSource::Scope(scope),
                            file: file_id,
                        }),
                        parent.module,
                    )
                };
                let r#in = self
                    .add_element(
                        ElementKey::Name(r#in),
                        scope,
                        ElementAuthored::Value(Value::In {
                            scope: scope.get_id(),
                            r#type: None,
                        }),
                    )
                    .ok()?;

                let function = erase_mut(self)
                    .get_module_local_mut(scope.module)
                    .pools
                    .functions
                    .insert(Function::new(
                        scope.get_id(),
                        r#in,
                        scope.module,
                        MaybeUninit::uninit().assume_init(),
                    ));
                let complete = self
                    .add_element(
                        ElementKey::Temp,
                        scope,
                        ElementAuthored::Expr(Expr::FunctionOptimized(function.get_id())),
                    )
                    .ok()?;
                function.complete = complete;
                Expr::Value(StaticValue::Function(function.get_id()))
            }
            _ => Expr::Value(StaticValue::Err),
        };
        Some(expr)
    }
    /// # Safety
    /// `location` is local
    unsafe fn diagnose(&mut self, location: Location, diagnoistic: Diagnostic) {
        match location {
            Location::Element(local_element_id) => {
                let element = unsafe { self.get_local_mut(local_element_id) };
                element.diagnoistics.push(diagnoistic);
            }
            Location::Scope(local_scope_id) => {
                let scope = unsafe { self.get_local_mut(local_scope_id) };
                scope.diagnoistics.push(diagnoistic);
            }
        }
    }
    /// # Safety
    /// `location` is local.
    unsafe fn grammar_error<T>(
        &mut self,
        location: Location,
        result: NodeResult<'static, T>,
    ) -> Option<T> {
        match result {
            Ok(source) => Some(source),
            Err(err) => {
                unsafe {
                    self.diagnose(
                        location,
                        Diagnostic::GrammarError {
                            source: err.node.upcast(),
                        },
                    )
                };
                None
            }
        }
    }
    /// # Safety
    /// - `module_id` is local.
    unsafe fn run_module(&mut self, module_id: ModuleId) {
        log::error!("run module(: {:?}", module_id);
        let module = unsafe { erase_mut(self).get_module_local_mut(module_id) };
        if module.has_runed() {
            return;
        }
        if let Some(authored) = module.authored {
            let root_scope = unsafe { self.add_scope(None, Some(authored), module_id) }.get_id();
            self.set_root_scope(module_id, root_scope);
            for element in module.pools.get::<Element>().iter() {
                unsafe {
                    // SAFETY: local: `module` -> `element`
                    self.run_element(element.get_id())
                };
            }
            module.unresolved_count -= 1;
            for dependant in mem::take(&mut module.dependants) {
                self.resolve_element(dependant);
            }
            self.decrease_workload();
        }
        log::error!("run module): {:?}", module_id);
    }
    /// # Safety
    /// - `element_id` is local.
    unsafe fn run_element(&mut self, element_id: Id<Element>) {
        let mut expr = {
            let element_local = unsafe { self.get_local_mut(element_id) };

            log::error!(
                "{element_id:?}: resolved: {}, dependencies_len: {}",
                element_local.is_resolved(),
                element_local.dependants.len()
            );

            if element_local.is_running {
                element_local.is_running = true;
                return;
            } else {
                element_local.is_running = true;
            }

            if element_local.is_resolved() || element_local.dependency_count > 0 {
                element_local.is_running = false;
                return;
            }

            let Some(expr) = element_local.expr else {
                element_local.is_running = false;
                return;
            };

            expr
        };

        let resolved_value = self.run_value(&mut expr, element_id);

        {
            let element_local = unsafe { self.get_local_mut(element_id) };
            element_local.expr = Some(expr);
            if element_local.dependency_count > 0 {
                element_local.is_running = false;
                return;
            }
        }

        self.set_element_value(
            element_id,
            resolved_value.unwrap_or(Value::Static(StaticValue::Err)),
        );
        let module =
            unsafe { self.get_module_local_mut(self.get(self.get(element_id).scope).module) };
        module.unresolved_count -= 1;
        let unresolved_count = module.unresolved_count;
        log::error!("{element_id:?}: run: {}", unresolved_count);

        let element_local = unsafe { self.get_local_mut(element_id) };
        log::error!(
            "{element_id:?}: resolved: {}, dependencies_len: {}",
            element_local.is_resolved(),
            element_local.dependants.len()
        );
        for dependant in mem::take(&mut element_local.dependants) {
            self.resolve_element(dependant.element_id);
        }
        let element_local = unsafe { self.get_local_mut(element_id) };
        element_local.is_running = false;
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn run_value(&mut self, expr: &mut Expr, element_id: Id<Element>) -> Option<Value> {
        let scope = self.get(element_id).scope;
        let module = self.get(scope).module;
        let expr = match *expr {
            Expr::Value(value) => Value::Static(value),
            Expr::Ref { element, source } => {
                self.depend_element_value(element_id, element, Some(source))?
            }
            Expr::Find { name, source } => {
                if let Some(ref_element_id) = self.find_element(scope, name, true) {
                    *expr = Expr::Ref {
                        element: ref_element_id,
                        source: source.upcast(),
                    };
                    self.depend_element_value(element_id, ref_element_id, Some(source.upcast()))?
                } else {
                    unsafe {
                        self.diagnose(
                            Location::Element(element_id),
                            Diagnostic::FailedFindElement {
                                source: source.upcast(),
                            },
                        )
                    };
                    return None;
                }
            }
            Expr::MetaFind { name, source } => {
                if let Some(ref_element_id) = self.find_element(scope, name, true) {
                    Value::Static(StaticValue::Element(ref_element_id))
                } else {
                    unsafe {
                        self.diagnose(
                            Location::Element(element_id),
                            Diagnostic::FailedFindElement {
                                source: source.upcast(),
                            },
                        )
                    };
                    return None;
                }
            }
            Expr::FindIn {
                value: ref_element_id,
                key,
                key_source,
                source,
            } => {
                let value = self.depend_child_element_value(element_id, ref_element_id)?;
                match value {
                    Value::Static(value) => match value {
                        StaticValue::Scope(scope_id) => {
                            log::error!("find element {}", &*self.id2str(key));
                            if let Some(find_element_id) = self.find_element(scope_id, key, false) {
                                *expr = Expr::Ref {
                                    element: find_element_id,
                                    source: key_source.upcast(),
                                };
                                self.depend_element_value(
                                    element_id,
                                    find_element_id,
                                    Some(key_source.upcast()),
                                )?
                            } else {
                                unsafe {
                                    self.diagnose(
                                        Location::Element(element_id),
                                        Diagnostic::FailedFindElement {
                                            source: key_source.upcast(),
                                        },
                                    )
                                };
                                return None;
                            }
                        }
                        _ => return None,
                    },
                    _ => return None,
                }
            }
            Expr::MetaFindIn {
                value: ref_element_id,
                key,
                key_source,
                source,
            } => {
                let value = self.depend_child_element_value(element_id, ref_element_id)?;
                match value {
                    Value::Static(value) => match value {
                        StaticValue::Scope(scope_id) => {
                            log::error!("find element {}", &*self.id2str(key));
                            if let Some(find_element_id) = self.find_element(scope_id, key, false) {
                                Value::Static(StaticValue::Element(find_element_id))
                            } else {
                                unsafe {
                                    self.diagnose(
                                        Location::Element(element_id),
                                        Diagnostic::FailedFindElement {
                                            source: key_source.upcast(),
                                        },
                                    )
                                };
                                return None;
                            }
                        }
                        _ => return None,
                    },
                    _ => return None,
                }
            }
            Expr::Call {
                func,
                param: param_id,
                source,
            } => {
                let func = self
                    .depend_child_element_value(element_id, func)
                    .expect(&format!("{:?}", self.get(func)));
                let param = self
                    .depend_child_element_value(element_id, param_id)
                    .expect(&format!("{:?}", self.get(param_id)));
                match func {
                    Value::Static(value) => match value {
                        StaticValue::Builtin(builtin) => match builtin {
                            Builtin::Mod => {
                                if let Some(scope) = merge_in!(self, param) {
                                    return Some(Value::In {
                                        scope,
                                        r#type: Some(Type {
                                            value: StaticValue::ScopeTy,
                                            depth: 0,
                                        }),
                                    });
                                }
                                let path = *param.as_static().ok()?.as_string().ok()?;
                                let str = erase(self).id2str(path);
                                let path = Path::new(SRC_PATH)
                                    .join(str.deref())
                                    .with_extension(SRC_FILE_EXTENSION);
                                let abs_path = self.get_worksapce_path().join(&path);
                                if !abs_path.exists() {
                                    let param = self.get(param_id);
                                    unsafe {
                                        self.diagnose(
                                            Location::Element(element_id),
                                            Diagnostic::PathError {
                                                source: param
                                                    .source
                                                    .as_ref()
                                                    .unwrap()
                                                    .value_source
                                                    .upcast(),
                                            },
                                        )
                                    };
                                    return None;
                                }
                                let module_id = self.depend_module(path, element_id)?;
                                if self.is_local_module(module_id) {
                                    unsafe { self.run_module(module_id) };
                                };
                                let scope_id = if self.is_remote_module(module_id) {
                                    let module = self.get_module(module_id);
                                    *module.root_scope.get()?
                                } else {
                                    let module = unsafe { self.get_module_local(module_id) };
                                    module.root_scope?
                                };

                                Value::Static(StaticValue::Scope(scope_id))
                            }
                            Builtin::Diagnose => {
                                let scope = *param.as_static().ok()?.as_scope().ok()?;
                                let on_key = self.str2id("on");
                                let source_key = self.str2id("source");
                                let text_key = self.str2id("text");

                                let on = *self
                                    .depend_element_value(
                                        element_id,
                                        self.find_element(scope, on_key, false)?,
                                        Some(source.upcast()),
                                    )?
                                    .as_static()
                                    .ok()?
                                    .as_int()
                                    .ok()?;
                                let text = *self
                                    .depend_element_value(
                                        element_id,
                                        self.find_element(scope, text_key, false)?,
                                        Some(source.upcast()),
                                    )?
                                    .as_static()
                                    .ok()?
                                    .as_string()
                                    .ok()?;
                                let source_element = *self
                                    .depend_element_value(
                                        element_id,
                                        self.find_element(scope, source_key, false)?,
                                        Some(source.upcast()),
                                    )?
                                    .as_static()
                                    .ok()?
                                    .as_element()
                                    .ok()?;
                                if on != 0 && self.is_local(source_element) {
                                    unsafe {
                                        self.diagnose(
                                            Location::Element(source_element),
                                            Diagnostic::Custom {
                                                source: self
                                                    .get(source_element)
                                                    .source
                                                    .as_ref()
                                                    .unwrap()
                                                    .key_source
                                                    .unwrap()
                                                    .upcast(),
                                                text,
                                            },
                                        )
                                    };
                                }
                                Value::Static(StaticValue::Trivial)
                            }
                            _ => return None,
                        },
                        StaticValue::Function(function) => {
                            let function = erase(self).get(function);
                            let _ = self
                                .depend_child_element_value(element_id, function.complete)
                                .expect(&format!("{:?}", self.get(function.complete)));
                            let optimized = unsafe { function.optimized.as_ref_unchecked() };
                            struct Context<'a, IP: ?Sized> {
                                interpreter: &'a mut IP,
                                optimized: &'a FunctionOptimized,
                                parent: Id<Scope>,
                                module: ModuleId,
                                element_map: Vec<Option<Id<Element>>>,
                                scope_map: Vec<Option<Id<Scope>>>,
                                param_id: Id<Element>,
                            }
                            let mut ctx = Context {
                                interpreter: self,
                                optimized,
                                parent: scope,
                                module,
                                element_map: Default::default(),
                                scope_map: Default::default(),
                                param_id,
                            };
                            impl<'a, IP: InterpreterLikeMut + ?Sized> Context<'a, IP> {
                                fn instantiate_scope(&mut self, id: Id<Scope>) -> Id<Scope> {
                                    if let Some(id) = self.scope_map.get(id.0).copied().flatten() {
                                        return id;
                                    }
                                    let scope = unsafe {
                                        erase_mut(self).interpreter.add_scope(
                                            Some(self.parent),
                                            None,
                                            self.module,
                                        )
                                    };
                                    let scope_id = scope.get_id();
                                    let function_scope = &self.optimized.scopes[id.0];
                                    for element in function_scope.elements.iter().copied() {
                                        self.instantiate_element(scope, element);
                                    }
                                    if self.scope_map.len() <= id.0 {
                                        self.scope_map.resize(id.0 + 1, Default::default());
                                    }
                                    self.scope_map[id.0] = Some(scope_id);
                                    scope_id
                                }
                                fn instantiate_element(
                                    &mut self,
                                    scope: &mut Scope,
                                    id: Id<Element>,
                                ) -> Id<Element> {
                                    if id == IN_OPTIMIZED {
                                        return self.param_id;
                                    }
                                    if let Some(id) = self.element_map.get(id.0).copied().flatten()
                                    {
                                        return id;
                                    }
                                    let function_element = &self.optimized.elements[id.0];
                                    let authored = match function_element.authored {
                                        FunctionElementAuthored::Expr(expr) => {
                                            ElementAuthored::Expr(
                                                expr.map_ref(|id| {
                                                    self.instantiate_element(scope, id)
                                                }),
                                            )
                                        }
                                        FunctionElementAuthored::Value(value) => {
                                            let value = match *value.as_static().unwrap() {
                                                StaticValue::Scope(id) => Value::Static(
                                                    StaticValue::Scope(self.instantiate_scope(id)),
                                                ),
                                                _ => value,
                                            };
                                            ElementAuthored::Value(value)
                                        }
                                    };
                                    let new_id = self
                                        .interpreter
                                        .add_element(function_element.key, scope, authored)
                                        .unwrap();
                                    if self.element_map.len() <= id.0 {
                                        self.element_map.resize(id.0 + 1, Default::default());
                                    }
                                    self.element_map[id.0] = Some(new_id);
                                    new_id
                                }
                            }
                            Value::Static(StaticValue::Scope(
                                ctx.instantiate_scope(optimized.root_scope.unwrap()),
                            ))
                        }
                        _ => return None,
                    },
                    _ => {
                        return None;
                    }
                }
            }
            Expr::FunctionOptimized(id) => {
                let function = erase(self).get(id);
                let scope = erase(self).get(function.scope);

                struct ResolveContext<'a, IP: ?Sized> {
                    interpreter: &'a mut IP,
                    resolved: bool,
                    element_id: Id<Element>,
                    function: &'a Function,
                    scope: &'a Scope,
                    visited_elements: HashSet<Id<Element>>,
                    visited_scope: HashSet<Id<Scope>>,
                }
                let mut ctx = ResolveContext {
                    interpreter: self,
                    resolved: true,
                    element_id,
                    function,
                    scope,
                    visited_elements: Default::default(),
                    visited_scope: Default::default(),
                };
                impl<'a, IP: InterpreterLikeMut + ?Sized> ResolveContext<'a, IP> {
                    fn visit(&mut self, id: Id<Element>) {
                        if id == self.function.r#in {
                            return;
                        }
                        if let Some(value) = self
                            .interpreter
                            .depend_child_element_value(self.element_id, id)
                        {
                            if let Value::Static(value) = value {
                                if let StaticValue::Scope(scope) = value {
                                    self.resolve_scope(scope);
                                }
                            }
                        } else {
                            self.resolved = false;
                        }
                        self.visited_elements.insert(id);
                    }
                    fn if_terminate(&self, id: Id<Element>) -> bool {
                        let element = unsafe { self.interpreter.get_local(id) };
                        if let Some(value) = element.value {
                            if let Value::In { scope, r#type } = value {
                                scope != self.function.scope
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    }
                    fn visited(&self, id: Id<Element>) -> bool {
                        self.visited_elements.contains(&id)
                    }
                    fn resolve_element(&mut self, element: Id<Element>) {
                        erase(self).interpreter.traverse(
                            self,
                            Self::visit,
                            Self::if_terminate,
                            Self::visited,
                            element,
                        );
                    }
                    fn resolve_scope(&mut self, scope: Id<Scope>) {
                        if self.visited_scope.insert(scope) {
                            let scope = erase(self).interpreter.get(scope);
                            for element in scope.elements.values().copied() {
                                self.resolve_element(element);
                            }
                        }
                    }
                }
                ctx.resolve_scope(function.scope);
                if !ctx.resolved {
                    return None;
                }

                let optimized = unsafe { function.optimized.as_mut_unchecked() };
                struct CollectContext<'a, IP: ?Sized> {
                    interpreter: &'a IP,
                    function: &'a Function,
                    optimized: &'a mut FunctionOptimized,
                    scope: &'a Scope,
                    element_map: HashMap<Id<Element>, Id<Element>>,
                    scope_map: HashMap<Id<Scope>, Id<Scope>>,
                }
                let mut ctx = CollectContext {
                    interpreter: self,
                    function,
                    optimized,
                    scope,
                    element_map: Default::default(),
                    scope_map: Default::default(),
                };
                impl<'a, IP: InterpreterLikeMut + ?Sized> CollectContext<'a, IP> {
                    fn collect(&mut self, id: Id<Element>, expr: Option<Expr>) -> Id<Element> {
                        if id == self.function.r#in {
                            return IN_OPTIMIZED;
                        }
                        let function_element = FunctionElement {
                            authored: {
                                let element = unsafe { self.interpreter.get_local(id) };
                                let value = element.value.unwrap();
                                match value {
                                    Value::In { scope, .. } => {
                                        if scope == self.scope.get_id() {
                                            FunctionElementAuthored::Expr(
                                                expr.unwrap_or(element.expr.unwrap()),
                                            )
                                        } else {
                                            FunctionElementAuthored::Value(value)
                                        }
                                    }
                                    Value::Static(value) => match value {
                                        StaticValue::Scope(id) => {
                                            let id = self.run_scope(id);
                                            FunctionElementAuthored::Value(Value::Static(
                                                StaticValue::Scope(id),
                                            ))
                                        }
                                        _ => FunctionElementAuthored::Value(Value::Static(value)),
                                    },
                                }
                            },
                            key: self.interpreter.get(id).key,
                        };
                        self.optimized.elements.push(function_element);
                        let new_id = Id::from_idx(self.optimized.elements.len() - 1);
                        debug_assert!(self.element_map.insert(id, new_id).is_none());
                        new_id
                    }
                    fn if_terminate(&self, id: Id<Element>) -> bool {
                        if let Value::In { scope, r#type } =
                            unsafe { self.interpreter.get_local(id).value.unwrap() }
                        {
                            scope != self.function.scope
                        } else {
                            true
                        }
                    }
                    fn element_map(&self, id: Id<Element>) -> Option<Id<Element>> {
                        self.element_map.get(&id).copied()
                    }
                    fn run_scope(&mut self, scope: Id<Scope>) -> Id<Scope> {
                        if let Some(scope) = self.scope_map.get(&scope).copied() {
                            return scope;
                        }
                        let mut elements = vec![];
                        let scope = self.interpreter.get(scope);
                        for element in scope.elements.values().copied() {
                            elements.push(self.run_element(element));
                        }
                        self.optimized.scopes.push(FunctionScope { elements });
                        let new_id = Id::from_idx(self.optimized.scopes.len() - 1);
                        self.scope_map.insert(scope.get_id(), new_id);
                        new_id
                    }
                    fn run_element(&mut self, element: Id<Element>) -> Id<Element> {
                        self.interpreter.collect(
                            self,
                            Self::collect,
                            Self::if_terminate,
                            Self::element_map,
                            element,
                        )
                    }
                }
                optimized.root_scope = Some(ctx.run_scope(function.scope));
                Value::Static(StaticValue::FunctionOptimized(function.get_id()))
            }
            _ => return None,
        };

        Some(expr)
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    /// - element's value has been resolved.
    fn set_element_value(&mut self, element_id: Id<Element>, value: Value) {
        let element_local = unsafe { self.get_local_mut(element_id) };
        element_local.value = Some(value);
        if self.is_concurrent() {
            let element = self.get(element_id);
            element.value.set(value);
        } else {
            let element = unsafe { self.get_mut(element_id) };
            element.value = OnceLock::from(value);
        }
    }
    /// # Panic
    /// when concurrent, element is not in local thread.
    fn depend_module_raw(&mut self, path: PathBuf, element_id: Id<Element>) -> Option<ModuleId>;
    /// # Panic
    /// when concurrent, element is not in local thread.
    fn depend_module(&mut self, path: PathBuf, element_id: Id<Element>) -> Option<ModuleId> {
        if let Some(file) = self.find_file(&path) {
            let file = self.get_file(file);
            if let Some(module) = file.is_module {
                return Some(module);
            }
        }
        self.depend_module_raw(path, element_id)
    }
    /// # Panic
    /// - when concurrent, dependant is not in local thread.
    /// - when not concurrent, dependency id is remote.
    fn depend_element(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
        source: Option<UntypedNode<'static>>,
        local: bool,
    ) -> bool {
        if self.is_local(dependency_id) {
            unsafe { self.run_element(dependency_id) };
            let dependency = erase_mut(unsafe { self.get_local_mut(dependency_id) });
            if local {
                if dependency.is_resolved() {
                    return true;
                } else {
                    let dependant_local = unsafe { self.get_local_mut(dependant_id) };
                    dependant_local.dependency_count += 1;
                }
            } else {
                if dependency.is_resolved() {
                    self.resolve_element(dependency_id);
                    return true;
                }
            }
            dependency.dependants.push(Dependant {
                element_id: dependant_id,
                source,
            });
        } else {
            debug_assert!(local);
            if self.get(dependency_id).value.get().is_some() {
                return true;
            }
            let dependant_local = unsafe { self.get_local_mut(dependant_id) };
            dependant_local.dependency_count += 1;
            if let Some(thread) = self.get_thread_remote_of(dependency_id) {
                thread.channel.push(Signal::Depend(Depend {
                    dependant: dependant_id,
                    dependency: dependency_id,
                    source,
                }));
                self.increase_workload();
            }
        }
        false
    }
    /// # Panic
    /// - when concurrent, dependant is not in local thread.
    /// - when not concurrent, dependency use remote id.
    fn depend_element_value(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
        source: Option<UntypedNode<'static>>,
    ) -> Option<Value> {
        if self.depend_element(dependant_id, dependency_id, source, true) {
            Some(self.get_element_value(dependency_id).unwrap())
        } else {
            None
        }
    }
    /// # Panic
    /// - when concurrent, any element is not in local thread.
    fn depend_child_element_value(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
    ) -> Option<Value> {
        let dependency = self.get(dependency_id);
        let source = dependency.source.as_ref().map(|x| x.value_source.upcast());
        self.depend_element_value(dependant_id, dependency_id, source)
    }
    /// # Panic
    /// element is not in threads
    fn resolve_element(&mut self, id: Id<Element>) {
        if self.is_local(id) {
            let dependant = unsafe { self.get_local_mut(id) };
            dependant.dependency_count -= 1;
            unsafe { self.run_element(id) }
        } else {
            let thread = self.get_thread_remote_of(id).unwrap();
            thread.channel.push(Signal::Resolve(id));
            self.increase_workload();
        }
    }
    /// # Safety
    /// - `module_id` is local.
    /// - `scope_id` is in `module_id`.
    fn set_root_scope(&mut self, module_id: ModuleId, scope_id: Id<Scope>) {
        debug_assert!(self.get(scope_id).module == module_id);
        unsafe { self.get_module_local_mut(module_id) }.root_scope = Some(scope_id);
        if self.is_concurrent() {
            self.get_module(module_id).root_scope.set(scope_id).unwrap();
        } else {
            unsafe { self.get_module_mut(module_id) }.root_scope = OnceLock::from(scope_id);
        }
    }
}

impl InterpreterLike for Interpreter {
    fn id2str(&self, id: StringId) -> impl Deref<Target = str> {
        self.strings.resolve(id)
    }
    fn is_concurrent(&self) -> bool {
        self.is_concurrent
    }
    fn is_local_module(&self, _module: ModuleId) -> bool {
        !self.is_concurrent()
    }
    fn is_remote_module(&self, id: ModuleId) -> bool {
        if self.is_concurrent() {
            self.concurrent.module2thread.contains_key(id)
        } else {
            false
        }
    }
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote {
        &self.concurrent.threads.get(id).remote
    }
    fn get_thread_remote_of_module(&self, module: ModuleId) -> Option<&ThreadRemote> {
        if let Some(id) = self.concurrent.module2thread.get(module).copied() {
            Some(self.get_thread_remote(id))
        } else {
            None
        }
    }
    fn get_module(&self, id: ModuleId) -> &Module {
        self.modules.get(id).unwrap()
    }
    unsafe fn get_module_local(&self, id: ModuleId) -> &ModuleLocal {
        debug_assert!(!self.is_remote_module(id));
        unsafe { self.get_module(id).local.as_ref_unchecked() }
    }
    fn get_file(&self, id: FileId) -> &File {
        &self.files[id]
    }

    fn get_worksapce_path(&self) -> &Path {
        &self.workspace_path
    }

    fn find_file(&self, path: impl AsRef<Path>) -> Option<FileId> {
        self.path2file.get(path.as_ref()).copied()
    }

    fn get_builtin_module(&self) -> ModuleId {
        self.builtin_module.unwrap()
    }
}

impl<'a, IP: Deref<Target = Interpreter>> InterpreterLike for ThreadedInterpreter<'a, IP> {
    fn is_concurrent(&self) -> bool {
        true
    }

    fn is_local_module(&self, module: ModuleId) -> bool {
        Some(self.thread)
            == self
                .interpreter
                .concurrent
                .module2thread
                .get(module)
                .copied()
    }

    fn is_remote_module(&self, id: ModuleId) -> bool {
        if let Some(id) = self.interpreter.concurrent.module2thread.get(id).copied() {
            id != self.thread
        } else {
            false
        }
    }
    fn id2str(&self, id: StringId) -> impl Deref<Target = str> {
        self.interpreter.concurrent.strings.resolve(id)
    }
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote {
        self.interpreter.get_thread_remote(id)
    }
    fn get_thread_remote_of_module(&self, module: ModuleId) -> Option<&ThreadRemote> {
        self.interpreter.get_thread_remote_of_module(module)
    }
    fn get_module(&self, id: ModuleId) -> &Module {
        self.interpreter.get_module(id)
    }
    unsafe fn get_module_local(&self, id: ModuleId) -> &ModuleLocal {
        debug_assert!(!self.is_remote_module(id));
        unsafe { self.get_module(id).local.as_ref_unchecked() }
    }
    fn get_file(&self, id: FileId) -> &File {
        &self.interpreter.files[id]
    }

    fn get_worksapce_path(&self) -> &Path {
        self.interpreter.get_worksapce_path()
    }

    fn find_file(&self, path: impl AsRef<Path>) -> Option<FileId> {
        self.interpreter.find_file(path)
    }

    fn get_builtin_module(&self) -> ModuleId {
        self.interpreter.get_builtin_module()
    }
}

impl InterpreterLikeMut for Interpreter {
    fn str2id(&mut self, str: &str) -> StringId {
        self.strings.get_or_intern(str)
    }

    fn get_thread_local_mut(&mut self, id: ThreadId) -> &mut ThreadLocal {
        self.concurrent.threads.get_mut(id).local.get_mut()
    }

    fn get_thread_local_mut_of(&mut self, module: ModuleId) -> Option<&mut ThreadLocal> {
        if let Some(id) = self.concurrent.module2thread.get(module).copied() {
            Some(self.get_thread_local_mut(id))
        } else {
            None
        }
    }

    unsafe fn get_module_mut_raw(&mut self, id: ModuleId) -> &mut Module {
        self.modules.get_mut(id).unwrap()
    }

    fn depend_module_raw(&mut self, path: PathBuf, element_id: Id<Element>) -> Option<ModuleId> {
        Some(self.add_module(Some(path)))
    }

    fn increase_workload(&mut self) {
        let workload = self.concurrent.workload.get_mut();
        *workload += 1;
        log::error!("inc workload: {}", workload);
    }

    fn decrease_workload(&mut self) -> usize {
        let workload = self.concurrent.workload.get_mut();
        *workload -= 1;
        log::error!("dec workload: {}", workload);
        *workload
    }
}

impl<'a, IP: Deref<Target = Interpreter>> InterpreterLikeMut for ThreadedInterpreter<'a, IP> {
    fn str2id(&mut self, str: &str) -> StringId {
        self.interpreter.concurrent.strings.get_or_intern(str)
    }

    fn get_thread_local_mut(&mut self, id: ThreadId) -> &mut ThreadLocal {
        assert!(id == self.thread);
        unsafe {
            self.interpreter
                .concurrent
                .threads
                .get(id)
                .local
                .as_mut_unchecked()
        }
    }

    fn get_thread_local_mut_of(&mut self, module: ModuleId) -> Option<&mut ThreadLocal> {
        if let Some(id) = self
            .interpreter
            .concurrent
            .module2thread
            .get(module)
            .copied()
        {
            Some(self.get_thread_local_mut(id))
        } else {
            None
        }
    }

    unsafe fn get_module_mut_raw(&mut self, _id: ModuleId) -> &mut Module {
        unreachable!()
    }

    fn depend_module_raw(&mut self, path: PathBuf, element_id: Id<Element>) -> Option<ModuleId> {
        let element = unsafe { self.get_local_mut(element_id) };
        element.dependency_count += 1;
        let add_module_delay = &mut self.get_thread_local_mut(self.thread).add_module_delay;
        add_module_delay
            .files
            .entry(path)
            .or_default()
            .push(element_id);
        None
    }

    fn increase_workload(&mut self) {
        let ret = self
            .interpreter
            .concurrent
            .workload
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        log::error!("inc workload: {}", ret);
    }

    fn decrease_workload(&mut self) -> usize {
        let ret = self
            .interpreter
            .concurrent
            .workload
            .fetch_sub(1, Ordering::Relaxed)
            - 1;
        if ret == 0 {
            self.interpreter.concurrent.workload_zero.notify_waiters();
        }
        log::error!("dec workload: {}", ret);
        ret
    }
}
