use crate::erase_struct;
use crate::interpreter::diagnose::Diagnostic;
use crate::interpreter::element::Dependant;
use crate::interpreter::element::Element;
use crate::interpreter::element::ElementAuthored;
use crate::interpreter::element::ElementKey;
use crate::interpreter::element::ElementSource;
use crate::interpreter::expr::Expr;
use crate::interpreter::expr::HasRef as _;
use crate::interpreter::file::File;
use crate::interpreter::file::FileId;
use crate::interpreter::module::Module;
use crate::interpreter::module::ModuleId;
use crate::interpreter::module::ModuleLocal;
use crate::interpreter::module::Pools;
use crate::interpreter::parse::parse_value;
use crate::interpreter::scope::Scope;
use crate::interpreter::scope::ScopeAuthored;
use crate::interpreter::scope::ScopeSource;
use crate::interpreter::thread::Depend;
use crate::interpreter::thread::Signal;
use crate::interpreter::thread::Thread;
use crate::interpreter::thread::ThreadId;
use crate::interpreter::thread::ThreadLocal;
use crate::interpreter::thread::ThreadRemote;
use crate::interpreter::value::BuiltinFunction;
use crate::interpreter::value::Value;
use crate::utils::concurrent_string_interner::ConcurentInterner;
use crate::utils::concurrent_string_interner::StringId;
use crate::utils::erase;
use crate::utils::erase_mut;
use crate::utils::pool::InPool;
use crate::utils::secondary_linked_list::List;
use crate::utils::unsafe_cell::UnsafeCell;
use slotmap::SecondaryMap;
use slotmap::SlotMap;
use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
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
pub use type_sitter::Node;
use type_sitter::NodeResult;
pub use type_sitter::UntypedNode;
pub type Tree = type_sitter::Tree<moss::SourceFile<'static>>;
use crate::utils::typed_key::Vec as KeyVec;

pub mod diagnose;
pub mod element;
pub mod expr;
pub mod file;
pub mod function;
pub mod module;
pub mod scope;
pub mod thread;
pub mod value;

mod parse;
mod run;

pub const SRC_FILE_EXTENSION: &str = "moss";
pub const SRC_PATH: &str = "src";

pub struct Id<T>(*mut T);

unsafe impl<T> Send for Id<T> {}

impl<T> Id<T> {
    pub const ANY: Self = Self::from_idx(0);
    pub const fn from_idx(idx: usize) -> Self {
        Self(idx as *mut T)
    }
    pub fn from_ptr(ptr: *const T) -> Self {
        Self(ptr as *mut T)
    }
    pub fn to_idx(self) -> usize {
        self.0 as usize
    }
    pub fn to_ptr(self) -> *mut T {
        self.0
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
        Self(self.0.clone())
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
            Some(ElementAuthored::Value(Value::BuiltinFunction(
                BuiltinFunction::Mod,
            ))),
        )
        .unwrap();
        let diagnose_id = self.str2id("diagnose");
        self.add_element(
            ElementKey::Name(diagnose_id),
            scope,
            Some(ElementAuthored::Value(Value::BuiltinFunction(
                BuiltinFunction::Diagnose,
            ))),
        )
        .unwrap();
        self.set_element_value(
            self.get_module(module).root_scope.unwrap(),
            Value::Scope(value::Scope(scope.get_id())),
        );
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
        let builtin_scope = unsafe { erase_mut(self).add_scope(None, None, id) };
        let root_scope = self
            .add_element(ElementKey::Temp, builtin_scope, None)
            .unwrap();
        let module = self.modules.get_mut(id).unwrap();
        module.root_scope = Some(root_scope);
        id
    }
    pub async fn run(&mut self) {
        assert!(!self.is_concurrent);
        self.concurrent.module2thread.clear();
        self.concurrent.threads.clear();
        self.concurrent.strings.sync_from(&self.strings);
        let mut thread_num: usize = available_parallelism().unwrap().into();
        let mut module_num = self.unresolved_modules.len();
        if module_num == 0 {
            return;
        }
        log::error!(
            "run (: thread_num: {}, module_num: {}",
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
            unsafe { self.run_module(module_id) };
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
pub trait InterpreterLike: Sized {
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
    fn get_src_path(&self) -> PathBuf {
        self.get_worksapce_path().join(SRC_PATH)
    }
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
        unsafe { &*(id.to_ptr()) }
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
            let scope = self.get::<Scope>(
                self.get_element_value(self.get_module(builtin_module).root_scope.unwrap())
                    .unwrap()
                    .as_scope()
                    .unwrap()
                    .0,
            );
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
                    && let Some(expr) = &unsafe { self.interpreter.get_local(element_id) }.expr
                {
                    let mut expr = expr.clone();
                    expr.map_ref(|x| self.traverse(x));
                    Some(expr)
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
                    && let Some(expr) = &unsafe { self.interpreter.get_local(element_id) }.expr
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
        unsafe { &mut *(id.to_idx() as *mut T) }
    }
    unsafe fn add<T: InPool<Pools>>(&mut self, value: T, module_id: ModuleId) -> &mut T {
        let pool = T::get_mut(&mut unsafe { self.get_module_local_mut(module_id) }.pools);
        pool.insert(value)
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
                let element_authored = ElementAuthored::Source(ElementSource {
                    value_source: erase_struct!(value),
                    key_source: Some(erase_struct!(key)),
                });
                let _ = self.add_element(ElementKey::Name(name), scope, Some(element_authored));
            }
        }
        scope
    }
    fn add_element(
        &mut self,
        key: ElementKey,
        scope: &mut Scope,
        authored: Option<ElementAuthored>,
    ) -> Result<Id<Element>, ()> {
        struct Context<'a, IP: ?Sized> {
            interpreter: &'a mut IP,
            key: ElementKey,
            scope: &'a mut Scope,
            authored: Option<ElementAuthored>,
        }
        let mut ctx = Context {
            interpreter: self,
            key,
            scope,
            authored,
        };

        impl<'a, IP: InterpreterLikeMut + ?Sized> Context<'a, IP> {
            fn add_raw(&mut self) -> &mut Element {
                let value = unsafe { self.interpreter.get_module_local_mut(self.scope.module) }
                    .pools
                    .get_mut::<Element>()
                    .insert(Element::new(self.key, self.scope.get_id()));
                value
            }
            fn add(&mut self) -> Option<&mut Element> {
                match self.key {
                    ElementKey::Name(name) => match erase_mut(self).scope.elements.entry(name) {
                        std::collections::hash_map::Entry::Occupied(_occupied_entry) => {
                            if let Some(authored) = &self.authored {
                                if let ElementAuthored::Source(source) = authored {
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
        'unresolve: {
            if let Some(authored) = ctx.authored {
                match authored {
                    ElementAuthored::Source(source) => {
                        element.source = Some(source);
                        let expr = unsafe {
                            self.parse_value(Ok(source.value_source), element.get_id(), scope)
                        };
                        let element_local = element.local.get_mut();
                        element_local.expr = expr;
                    }
                    ElementAuthored::Value(value) => {
                        let element_local = element.local.get_mut();
                        element_local.value = Some(value);
                        element.value = OnceLock::from(value);
                        break 'unresolve;
                    }
                    ElementAuthored::Expr(expr) => {
                        let element_local = element.local.get_mut();
                        element_local.expr = Some(expr);
                    }
                }
            }
            let module = unsafe { self.get_module_local_mut(scope.module) };
            module.unresolved_count += 1;
            let unresolved_count = module.unresolved_count;
            log::error!("{:?}: add {}", element.get_id(), unresolved_count);
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
        scope: &mut Scope,
    ) -> Option<Expr> {
        parse_value(self, source, element_id, scope)
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
        let module_local = unsafe { erase_mut(self).get_module_local_mut(module_id) };
        let root_scope_element = self.get_module(module_id).root_scope.unwrap();
        if let Some(authored) = module_local.authored {
            let root_scope = unsafe { self.add_scope(None, Some(authored), module_id) }.get_id();
            self.set_element_value(root_scope_element, Value::Scope(value::Scope(root_scope)));
            for element in module_local.pools.get::<Element>().iter() {
                unsafe {
                    // SAFETY: local: `module` -> `element`
                    self.run_element(element.get_id())
                };
            }
            module_local.unresolved_count -= 1;
            for dependant in mem::take(&mut module_local.dependants) {
                self.resolve_element(dependant);
            }
            self.decrease_workload();
        }
        log::error!("run module): {:?}", module_id);
    }
    /// # Safety
    /// - `element_id` is local.
    unsafe fn run_element(&mut self, element_id: Id<Element>) {
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

        if element_local.expr.is_none() {
            element_local.is_running = false;
            return;
        };

        let resolved_value = self.run_value(element_id);

        {
            let element_local = unsafe { self.get_local_mut(element_id) };
            element_local.is_running = false;
            if element_local.dependency_count > 0 {
                return;
            }
        }

        self.set_element_value(
            element_id,
            resolved_value.unwrap_or(Value::Error(value::Error)),
        );
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn run_value(&mut self, element_id: Id<Element>) -> Option<Value> {
        run::Context::run_value(self, element_id)
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    /// - element's value has been resolved.
    fn set_element_value(&mut self, element_id: Id<Element>, value: Value) {
        let element_local = unsafe { self.get_local_mut(element_id) };
        element_local.value = Some(value);
        if self.is_concurrent() {
            let element = self.get(element_id);
            element.value.set(value).unwrap();
        } else {
            let element = unsafe { self.get_mut(element_id) };
            element.value = OnceLock::from(value);
        }

        let module =
            unsafe { self.get_module_local_mut(self.get(self.get(element_id).scope).module) };
        module.unresolved_count -= 1;
        let unresolved_count = module.unresolved_count;
        log::error!("{:?}: set {}", element_id, unresolved_count);

        let element_local = unsafe { self.get_local_mut(element_id) };
        for dependant in mem::take(&mut element_local.dependants) {
            self.resolve_element(dependant.element_id);
        }
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
