use crate::any_dyn;
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
use crate::interpreter::value::TypedValue;
use crate::interpreter::value::Value;
use crate::utils::concurrent_string_interner::ConcurentInterner;
use crate::utils::concurrent_string_interner::StringId;
use crate::utils::erase;
use crate::utils::erase_mut;
use crate::utils::secondary_linked_list::List;
use slotmap::SecondaryMap;
use slotmap::SlotMap;
use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::iter;
use std::marker::PhantomData;
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

pub struct Id<T>(pub usize,PhantomData<T>);

impl<T> Id<T>{
    pub fn new(ptr:*const T)->Self{
        Self(ptr as usize,Default::default())
    }
}

impl<T> Id<T>{
    fn with_ctx<'a,Ctx:InterpreterLike>(self,ctx:&'a Ctx)->ContextedId<'a,T,Ctx> where Self: Sized{
        ContextedId{
            id: self,
            ctx,
        }
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
        Id::new((self as *const Self) as *mut Self)
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
        Self(self.0.clone(),Default::default())
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

pub struct ContextedId<'a, T, Ctx> {
    id: Id<T>,
    ctx: &'a Ctx,
}

impl<'a, Ctx: InterpreterLike> Display for ContextedId<'a, Element, Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if let Some(value) = self.ctx.get_element_value(self.id) {
                value.value
            } else {
                Value::Err
            }
            .with_ctx(self.ctx)
        )
    }
}

impl<'a, Ctx: InterpreterLike> Display for ContextedId<'a, Scope, Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scope = self.ctx.get(self.id);
        write!(f, "{{")?;
        for (key, element) in &scope.elements {
            write!(
                f,
                "{}: {}, ",
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

unsafe impl Sync for Interpreter{}

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
        let elements = [
            ElementDescriptor {
                key: ElementKey::Name(self.str2id("mod")),
                value: TypedValue {
                    value: Value::Builtin(Builtin::Mod),
                    r#type: Value::Err,
                },
            },
            ElementDescriptor {
                key: ElementKey::Name(self.str2id("diagnose")),
                value: TypedValue {
                    value: Value::Builtin(Builtin::Diagnose),
                    r#type: Value::Err,
                },
            },
            ElementDescriptor {
                key: ElementKey::Name(self.str2id("dyn")),
                value: TypedValue {
                    value: Value::Dyn,
                    r#type: Value::Dyn,
                },
            },
        ];
        let scope = unsafe { self.add_scope(None, None, module, elements.into_iter()) };
        self.set_root_scope(module, scope);
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
            for thread_id in erase(self).concurrent.threads.keys() {
                let thread = erase_mut(self).get_thread_local_mut(thread_id);
                for (path, dependants) in mem::take(&mut thread.add_module_delay.files) {
                    let module_id = self.add_module(Some(path));
                    let module = unsafe { self.get_module_local_mut(module_id) };
                    for dependant in dependants.iter().copied() {
                        module.dependants.push(dependant);
                    }
                }
            }
            log::error!("run loop)");
        }
        log::error!("run)");
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
                            unsafe { self.get_local_mut(depend.dependency)
                                .dependants
                                .push(Dependant {
                                    element_id: depend.dependant,
                                    source: depend.source,
                                }) };
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

    fn get_element_value(&self, id: Id<Element>) -> Option<TypedValue> {
        if self.is_remote(id) {
            self.get(id).value.get().copied()
        } else {
            let local = unsafe { self.get_local(id) };
            if local.resolved {
                Some(local.value)
            } else {
                None
            }
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
        elements: impl Iterator<Item = ElementDescriptor>,
    ) -> Id<Scope> {
        let (scope_ptr, scope) = unsafe {
            self.get_module_local_mut(module)
                .pools
                .get_mut::<Scope>()
                .insert(Scope::new(parent, authored, module))
        };
        let scope_id = Id::new(scope_ptr);
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
        for element in elements {
            self.add_element(
                element.key,
                scope,
                ElementAuthored::Value {
                    value: element.value,
                },
            )
            .unwrap();
        }
        scope_id
    }
    fn add_element(
        &mut self,
        key: ElementKey,
        scope: &mut Scope,
        authored: ElementAuthored,
    ) -> Result<Id<Element>, ()> {
        // SAFETY: local: scope -> module
        let module_id = scope.module;
        // SAFETY: mut ref -> local.
        let scope_id = scope.get_id();
        let add_raw = |self_: &mut Self| {
            let source = match authored {
                ElementAuthored::Source { source, file } => Some(source),
                ElementAuthored::Value { value } => None,
            };
            let (ptr, value) = unsafe { self_.get_module_local_mut(module_id) }
                .pools
                .get_mut::<Element>()
                .insert(Element::new(key, scope_id, source));
            Id::new(ptr)
        };
        let id = match key {
            ElementKey::Name(name) => match scope.elements.entry(name) {
                std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    if let ElementAuthored::Source { source, file } = authored {
                        let source = if let Some(key_source) = source.key_source {
                            key_source.upcast()
                        } else {
                            source.value_source.upcast()
                        };
                        unsafe {
                            self.diagnose(
                                Location::Scope(scope_id),
                                Diagnostic::RedundantElementKey { source },
                            )
                        };
                    }
                    return Err(());
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    let id = add_raw(self);
                    vacant_entry.insert(id);
                    id
                }
            },
            ElementKey::Temp => add_raw(self),
        };
        match authored {
            ElementAuthored::Source { source, file } => {
                let raw_value = unsafe {
                    self.parse_value(Ok(source.value_source), id, scope, file)
                        .unwrap_or(Value::Err)
                };
                let element = unsafe { self.get_local_mut::<Element>(id) };
                element.value.value = raw_value;
                unsafe { self.get_module_local_mut(scope.module).unresolved_count+=1 };
            }
            ElementAuthored::Value { value } => {
                let element = unsafe { self.get_local_mut::<Element>(id) };
                element.value = value;
                element.resolved = true;
            }
        }
        Ok(id)
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
    ) -> Option<Value> {
        let source = unsafe { self.grammar_error(Location::Element(element_id), source) }?;
        let source_child =
            unsafe { self.grammar_error(Location::Element(element_id), source.child()) }?;
        let element_local = erase_mut(unsafe { self.get_local_mut::<Element>(element_id) });
        let element = self.get::<Element>(element_id);
        let scope_id = element.scope;
        debug_assert!(scope_id == parent.get_id());
        let value = match source_child {
            moss::ValueChild::Bracket(bracket) => unsafe {
                self.parse_value(bracket.value(), element_id, parent, file_id)
                    .unwrap_or(Value::Err)
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
                Value::Call {
                    func: func_element,
                    param: param_element,
                    source: call,
                }
            }
            moss::ValueChild::Scope(scope_source) => Value::Scope(unsafe {
                // SAFETY: element -> scope
                self.add_scope(
                    Some(scope_id),
                    Some(ScopeAuthored {
                        source: ScopeSource::Scope(scope_source),
                        file: file_id,
                    }),
                    parent.module,
                    iter::empty(),
                )
            }),
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
                Value::FindRef {
                    value: element,
                    key: self.get_source_str_id(&name, file_id),
                    key_source: name,
                    source: find,
                }
            }
            moss::ValueChild::Int(int) => {
                Value::Int(self.get_source_str(&int, file_id).parse().unwrap())
            }
            moss::ValueChild::Name(name) => {
                let string_id = self.get_source_str_id(&name, file_id);
                Value::Ref {
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

                Value::String(self.str2id(value.as_ref().map(|x| x.as_ref()).unwrap_or("")))
            }
            moss::ValueChild::Meta(meta) => {
                let name = self.grammar_error(Location::Element(element_id), meta.name())?;
                let string_id = self.get_source_str_id(&name, file_id);
                Value::Meta {
                    name: string_id,
                    source: meta,
                }
            }
            _ => Value::Err,
        };
        Some(value)
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
            let root_scope =
                unsafe { self.add_scope(None, Some(authored), module_id, iter::empty()) };
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
        let element_local = erase_mut(unsafe { self.get_local_mut(element_id) });
        let element = erase(self.get(element_id));

        if element_local.resolved || element_local.dependency_count > 0 {
            return;
        }

        if let ElementKey::Name(name) = element.key {
            log::error!("run element(: {}", &*self.id2str(name),);
        } else {
            log::error!("run element(: {:?}", element_id);
        }

        if let Some(resolved_value) = self.run_value(element_local.value.value, element_id) {
            self.set_element_value(
                element_id,
                resolved_value,
                element_local.dependency_count == 0,
            );
        }

        if element_local.dependency_count > 0 {
            return;
        }

        element_local.resolved = true;
        let module = unsafe { self.get_module_local_mut(self.get(element.scope).module) };
        module.unresolved_count -= 1;
        if let ElementKey::Name(name) = element.key {
            log::error!("run element): {}", &*self.id2str(name));
        } else {
            log::error!("run element): {:?}", element_id);
        }
        for dependant in mem::take(&mut element_local.dependants) {
            self.resolve_element(dependant.element_id);
        }
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn run_value(&mut self, value: Value, element_id: Id<Element>) -> Option<TypedValue> {
        let scope = self.get(element_id).scope;
        let value = match value {
            Value::Int(x) => TypedValue {
                value: Value::Int(x),
                r#type: Value::IntTy,
            },
            Value::IntTy => TypedValue {
                value: Value::IntTy,
                r#type: Value::TyTy,
            },
            Value::String(id) => TypedValue {
                value: Value::String(id),
                r#type: Value::StringTy,
            },
            Value::StringTy => TypedValue {
                value: Value::StringTy,
                r#type: Value::TyTy,
            },
            Value::Builtin(builtin) => TypedValue {
                value: Value::Builtin(builtin),
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
            Value::Ref { name, source } => {
                if let Some(ref_element_id) = self.find_element(scope, name, true) {
                    self.depend_element_value(element_id, ref_element_id, source.upcast())?
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
            Value::Meta { name, source } => {
                if let Some(ref_element_id) = self.find_element(scope, name, true) {
                    TypedValue {
                        value: Value::Element(ref_element_id),
                        r#type: Value::ElementTy,
                    }
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
            Value::FindRef {
                value: ref_element_id,
                key,
                key_source,
                source,
            } => {
                let value = self.depend_child_element_value(element_id, ref_element_id)?;
                match value.value {
                    Value::Scope(scope_id) => {
                        log::error!("find element {}", &*self.id2str(key));
                        if let Some(find_element_id) = self.find_element(scope_id, key, false) {
                            self.depend_element_value(
                                element_id,
                                find_element_id,
                                key_source.upcast(),
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
                    _ => {
                        unsafe {
                            self.diagnose(
                                Location::Element(element_id),
                                Diagnostic::CanNotFindIn {
                                    source: source.upcast(),
                                    value: value.value,
                                },
                            )
                        };
                        return None;
                    }
                }
            }
            Value::FindMeta {
                value: ref_element_id,
                key,
                key_source,
                source,
            } => {
                let value = self.depend_child_element_value(element_id, ref_element_id)?;
                match value.value {
                    Value::Scope(scope_id) => {
                        log::error!("find element {}", &*self.id2str(key));
                        if let Some(find_element_id) = self.find_element(scope_id, key, false) {
                            TypedValue {
                                value: Value::Element(find_element_id),
                                r#type: Value::ElementTy,
                            }
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
                    _ => {
                        unsafe {
                            self.diagnose(
                                Location::Element(element_id),
                                Diagnostic::CanNotFindIn {
                                    source: source.upcast(),
                                    value: value.value,
                                },
                            )
                        };
                        return None;
                    }
                }
            }
            Value::Call {
                func,
                param: param_id,
                source,
            } => {
                let func = self.depend_child_element_value(element_id, func)?;
                let param = self.depend_child_element_value(element_id, param_id)?;
                match func.value {
                    Value::Builtin(builtin) => match builtin {
                        Builtin::Mod => {
                            if any_dyn!(&param.value) {
                                return Some(TypedValue {
                                    value,
                                    r#type: Value::ScopeTy,
                                });
                            }
                            match param.value {
                                Value::String(string_id) => {
                                    let str = erase(self).id2str(string_id);
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

                                    TypedValue {
                                        value: Value::Scope(scope_id),
                                        r#type: Value::ScopeTy,
                                    }
                                }
                                _ => return None,
                            }
                        }
                        Builtin::Diagnose => match param.value {
                            Value::Scope(scope_id) => {
                                let on_key = self.str2id("on");
                                let source_key = self.str2id("source");
                                let text_key = self.str2id("text");

                                let Value::Int(on) = self
                                    .depend_element_value(
                                        element_id,
                                        self.find_element(scope_id, on_key, false)?,
                                        source.upcast(),
                                    )?
                                    .value
                                else {
                                    return None;
                                };
                                let Value::String(text) = self
                                    .depend_element_value(
                                        element_id,
                                        self.find_element(scope_id, text_key, false)?,
                                        source.upcast(),
                                    )?
                                    .value
                                else {
                                    return None;
                                };
                                let Value::Element(source_element) = self
                                    .depend_element_value(
                                        element_id,
                                        self.find_element(scope_id, source_key, false)?,
                                        source.upcast(),
                                    )?
                                    .value
                                else {
                                    return None;
                                };
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
                                TypedValue {
                                    value: Value::Int(1),
                                    r#type: Value::IntTy,
                                }
                            }
                            _ => return None,
                        },
                        _ => return None,
                    },
                    _ => {
                        unsafe {
                            self.diagnose(
                                Location::Element(element_id),
                                Diagnostic::CanNotCallOn {
                                    source: source.upcast(),
                                    value,
                                },
                            )
                        };
                        return None;
                    }
                }
            }
            _ => return None,
        };

        Some(value)
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    /// - element's value has been resolved.
    fn set_element_value(&mut self, element_id: Id<Element>, value: TypedValue, resolved: bool) {
        let element_local = unsafe { self.get_local_mut(element_id) };
        element_local.value = value;
        element_local.resolved = resolved;
        if resolved {
            if self.is_concurrent() {
                let element = self.get(element_id);
                element.value.set(value);
            } else {
                let element = unsafe { self.get_mut(element_id) };
                element.value = OnceLock::from(value);
            }
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
        source: UntypedNode<'static>,
        local: bool,
    ) {
        if local {
            let dependant_local = unsafe { self.get_local_mut(dependant_id) };
            dependant_local.dependency_count += 1;
        }

        if self.is_local(dependency_id) {
            let dependency = erase_mut(unsafe { self.get_local_mut(dependency_id) });
            dependency.dependants.push(Dependant {
                element_id: dependant_id,
                source,
            });
        } else {
            if let Some(thread) = self.get_thread_remote_of(dependency_id) {
                thread.channel.push(Signal::Depend(Depend {
                    dependant: dependant_id,
                    dependency: dependency_id,
                    source,
                }));
                self.increase_workload();
            }
        }
    }
    /// # Panic
    /// - when concurrent, dependant is not in local thread.
    /// - when not concurrent, dependency use remote id.
    fn depend_element_value(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
        source: UntypedNode<'static>,
    ) -> Option<TypedValue> {
        let mut typed_value = self.get_element_value(dependency_id);
        if let Some(typed_value) = &mut typed_value {
            match typed_value.value {
                Value::Dyn => {
                    typed_value.value = Value::DynRef {
                        element: dependency_id,
                    }
                }
                _ => (),
            }
        } else {
            self.depend_element(dependant_id, dependency_id, source, true);
        }
        typed_value
    }
    /// # Panic
    /// - when concurrent, any element is not in local thread.
    fn depend_child_element_value(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
    ) -> Option<TypedValue> {
        let dependency = self.get(dependency_id);
        let source = dependency.source.as_ref().unwrap().value_source.upcast();
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
