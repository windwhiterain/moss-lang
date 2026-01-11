use crate::erase_struct;
use crate::interpreter::diagnose::Diagnostic;
use crate::interpreter::element::ConcurrentElementId;
use crate::interpreter::element::Dependant;
use crate::interpreter::element::Element;
use crate::interpreter::element::ElementAuthored;
use crate::interpreter::element::ElementDescriptor;
use crate::interpreter::element::ElementId;
use crate::interpreter::element::ElementKey;
use crate::interpreter::element::ElementRemote;
use crate::interpreter::element::ElementRemoteCell;
use crate::interpreter::element::ElementSource;
use crate::interpreter::element::InModuleElementId;
use crate::interpreter::element::RemoteElementId;
use crate::interpreter::element::RemoteInModuleElementId;
use crate::interpreter::file::File;
use crate::interpreter::file::FileId;
use crate::interpreter::module::ConcurrentModule;
use crate::interpreter::module::Module;
use crate::interpreter::module::ModuleId;
use crate::interpreter::module::ModuleRemote;
use crate::interpreter::scope::ConcurrentScopeId;
use crate::interpreter::scope::InModuleScopeId;
use crate::interpreter::scope::RemoteInModuleScopeId;
use crate::interpreter::scope::RemoteScopeId;
use crate::interpreter::scope::Scope;
use crate::interpreter::scope::ScopeAuthored;
use crate::interpreter::scope::ScopeId;
use crate::interpreter::scope::ScopeRemote;
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
use std::collections::HashMap;
use std::iter;
use std::mem;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
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
pub mod module;
pub mod scope;
pub mod thread;
pub mod value;

static SRC_FILE_EXTENSION: &str = "moss";
static SRC_PATH: &str = "src";

pub trait InModuleId {
    type GlobalId;
    fn global(self, module: ModuleId) -> Self::GlobalId;
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
    pub modules: SlotMap<ModuleId, ConcurrentModule>,
    pub unresolved_modules: List<ModuleId>,
    pub concurrent: InterpreterConcurrent,
    pub is_concurrent: bool,
    pub builtin_module: Option<ModuleId>,
}

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
        ];
        let scope = self.add_scope(None, None, module, elements.into_iter());
        self.set_root_scope(scope.global(module));
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

        let id = self
            .modules
            .insert(ConcurrentModule::new(authored, resolved));
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
                .retain(|key| self.get_module(key).is_resolved());
            for thread_id in erase(self).concurrent.threads.keys() {
                let thread = erase_mut(self).get_thread_local_mut(thread_id);
                for (path, dependants) in mem::take(&mut thread.add_module_delay.files) {
                    let module_id = self.add_module(Some(path));
                    let module = self.get_module_mut(module_id);
                    for dependant in dependants.iter().copied() {
                        module.dependants.push(dependant);
                    }
                }
            }
            log::error!("run loop)");
            for (module_id, module) in &mut self.modules {
                unsafe {
                    let scopes = &module.local.as_ref_unchecked().scopes;
                    let elements = &module.local.as_ref_unchecked().elements;
                    let remote_scopes = &module.remote.scopes;
                    let remote_elements = &module.remote.elements;
                    log::error!(
                        "module {:?}:\n scopes:\n{:?},\n elements:\n{:?},\n remmote_scopes:\n{:?},\n remote_elements:{:?}",
                        module_id,
                        scopes,
                        elements,
                        remote_scopes,
                        remote_elements
                    );
                }
            }
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
    fn is_module_local(&self, module: ModuleId) -> bool {
        Some(self.thread)
            == self
                .interpreter
                .concurrent
                .module2thread
                .get(module)
                .copied()
    }
    fn is_module_remote(&self, module: ModuleId) -> bool {
        if let Some(id) = self
            .interpreter
            .concurrent
            .module2thread
            .get(module)
            .copied()
        {
            id != self.thread
        } else {
            false
        }
    }
    async fn run(&mut self) {
        let modules = &mut erase_mut(self).get_thread_local_mut(self.thread).modules;
        log::error!(
            "run thread(: {:?}, modules_num: {}",
            self.thread,
            modules.len()
        );
        let mut unresolved_num = modules.len();
        for module_id in modules.iter().copied() {
            let module = erase_mut(self.get_module_mut(module_id));
            if !module.has_runed() {
                self.run_module(module_id);
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
                                ConcurrentElementId::Local(depend.dependency),
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
                            self.get_element_mut(depend.dependency)
                                .dependants
                                .push(Dependant {
                                    element_id: depend.dependant,
                                    source: depend.source,
                                });
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
    Element(ElementId),
    Scope(ScopeId),
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
    /// # Panic
    /// when concurrent, module is not in local thread.
    fn get_module_mut(&mut self, id: ModuleId) -> &mut Module;
    /// # Panic
    /// when concurrent, scope is not in local thread.
    fn get_scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        self.get_module_mut(id.module).scopes.get_mut(id.in_module)
    }
    /// # Panic
    /// when concurrent, element is not in local thread.
    fn get_element_mut(&mut self, id: ElementId) -> &mut Element {
        self.get_module_mut(id.module)
            .elements
            .get_mut(id.in_module)
    }
    /// # Panic
    /// when concurrent, element is not in local thread.
    fn add_scope_raw(&mut self, scope: Scope, module: ModuleId) -> InModuleScopeId {
        self.get_module_mut(module).scopes.insert(scope)
    }
    /// # Panic
    /// - when concurrent, module is not in local thread.
    fn add_scope(
        &mut self,
        parent: Option<InModuleScopeId>,
        authored: Option<ScopeAuthored>,
        module: ModuleId,
        elements: impl Iterator<Item = ElementDescriptor>,
    ) -> InModuleScopeId {
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
                log::error!("assign: {assign:?}");
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

                let name = self.get_source_str_id(&key, authored.file);
                let element_authored = ElementAuthored::Source {
                    source: ElementSource {
                        value_source: erase_struct!(value),
                        key_source: Some(erase_struct!(key)),
                    },
                    file: authored.file,
                };
                let _ = self.add_element(ElementKey::Name(name), scope_id, element_authored);
            }
        }
        for element in elements {
            self.add_element(
                element.key,
                scope_id,
                ElementAuthored::Value {
                    value: element.value,
                },
            )
            .unwrap();
        }
        self.publish_scope(scope_id);
        scope_id.in_module
    }
    /// # Panic
    /// - when concurrent, scope is not in local thread.
    fn publish_scope(&mut self, id: ScopeId) -> RemoteInModuleScopeId {
        let scope = erase_mut(self).get_scope_mut(id);
        assert!(self.is_module_local(id.module));
        let module_remote = erase_mut(self).get_module_remote(id.module);
        let mut remote_elements = HashMap::<StringId, RemoteInModuleElementId>::new();
        for (key, element_id) in &erase(scope).elements {
            let element = self.get_element_mut(element_id.global(id.module));
            let remote = ElementRemote::new(*element_id, element.resolved_value, element.resolved);
            let remote_id = unsafe { module_remote.elements.insert(remote) };
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
        let remote_id = unsafe { module_remote.scopes.insert(remote) };
        scope.remote_id = Some(remote_id);
        remote_id
    }
    /// # Panic
    /// - scope has not published.
    /// - when concurrent, module is not in local thread.
    fn set_root_scope(&mut self, id: ScopeId) {
        let module = self.get_module_mut(id.module);
        module.root_scope = Some(id.in_module);
        let scope = self.get_scope(id);
        let module_remote = self.get_module_remote(id.module);
        module_remote
            .root_scope
            .set(scope.remote_id.unwrap())
            .unwrap();
    }
    /// # Panic
    /// when concurrent, element is not in local thread.
    fn add_element_raw(&mut self, element: Element, module: ModuleId) -> InModuleElementId {
        let module = self.get_module_mut(module);
        module.unresolved_count += 1;
        module.elements.insert(element)
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn add_element(
        &mut self,
        key: ElementKey,
        scope_id: ScopeId,
        authored: ElementAuthored,
    ) -> Result<ElementId, ()> {
        let scope = erase_mut(self.get_scope_mut(scope_id));
        let new_id = |self_: &mut Self| {
            self_
                .add_element_raw(Element::new(key, scope_id.in_module), scope_id.module)
                .global(scope_id.module)
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
                        self.diagnose(
                            Location::Scope(scope_id),
                            Diagnostic::RedundantElementKey { source },
                        );
                    }
                    return Err(());
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    let id = new_id(self);
                    vacant_entry.insert(id.in_module);
                    id
                }
            },
            ElementKey::Temp => new_id(self),
        };
        match authored {
            ElementAuthored::Source { source, file } => {
                let raw_value = self
                    .parse_value(Ok(source.value_source), id, file)
                    .unwrap_or(Value::Err);
                let element = self.get_element_mut(id);
                element.raw_value = raw_value;
                element.authored = Some(source);
            }
            ElementAuthored::Value { value } => {
                let element = self.get_element_mut(id);
                element.resolved_value = value;
                element.resolved = true;
            }
        }
        Ok(id)
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn parse_value(
        &mut self,
        source: NodeResult<'static, moss::Value<'static>>,
        element_id: ElementId,
        file_id: FileId,
    ) -> Option<Value> {
        let source = self.grammar_error(Location::Element(element_id), source)?;
        let source_child = self.grammar_error(Location::Element(element_id), source.child())?;
        let element = erase_mut(self.get_element_mut(element_id));
        let scope_id = element.scope.global(element_id.module);
        let value = match source_child {
            moss::ValueChild::Bracket(bracket) => self
                .parse_value(bracket.value(), element_id, file_id)
                .unwrap_or(Value::Err),
            moss::ValueChild::Call(call) => {
                let func = self.grammar_error(Location::Element(element_id), call.func())?;
                let param = self.grammar_error(Location::Element(element_id), call.param())?;
                let func_element = self
                    .add_element(
                        ElementKey::Temp,
                        scope_id,
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
                        scope_id,
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
            moss::ValueChild::Scope(scope) => Value::Map(ConcurrentScopeId::from_local(
                erase(self),
                self.add_scope(
                    Some(scope_id.in_module),
                    Some(ScopeAuthored {
                        source: ScopeSource::Scope(scope),
                        file: file_id,
                    }),
                    element_id.module,
                    iter::empty(),
                )
                .global(element_id.module),
            )),
            moss::ValueChild::Find(find) => {
                let value = self.grammar_error(Location::Element(element_id), find.value())?;
                let name = self.grammar_error(Location::Element(element_id), find.name())?;
                let element = self
                    .add_element(
                        ElementKey::Temp,
                        scope_id,
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
                                _ => {
                                    erase_mut(self).diagnose(
                                        Location::Element(element_id),
                                        Diagnostic::StringEscapeError {
                                            source: string_escape.upcast(),
                                        },
                                    );
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
    /// # Panic
    /// - when concurrent, location is not in local thread.
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
    /// # Panic
    /// - when concurrent, location is not in local thread.
    fn grammar_error<T>(
        &mut self,
        location: Location,
        result: NodeResult<'static, T>,
    ) -> Option<T> {
        match result {
            Ok(source) => Some(source),
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
    /// # Panic
    /// when concurrent, module is not in local thread.
    fn run_module(&mut self, module_id: ModuleId) {
        log::error!("run module(: {:?}", module_id);
        let module = erase_mut(self).get_module_mut(module_id);
        if module.has_runed() {
            return;
        }
        if let Some(authored) = module.authored {
            let root_scope = self.add_scope(None, Some(authored), module_id, iter::empty());
            self.set_root_scope(root_scope.global(module_id));
            for element in module.elements.keys() {
                self.run_element(element.global(module_id));
            }
            module.unresolved_count -= 1;
            for dependant in mem::take(&mut module.dependants) {
                self.resolve_element(dependant);
            }
            self.decrease_workload();
        }
        log::error!("run module): {:?}", module_id);
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn run_element(&mut self, element_id: ElementId) {
        let element = erase_mut(self.get_element_mut(element_id));

        if element.resolved || element.dependency_count > 0 {
            return;
        }

        if let ElementKey::Name(name) = element.key {
            log::error!("run element(: {}", &*self.id2str(name),);
        } else {
            log::error!("run element(: {:?}", element_id.in_module,);
        }

        if let Some(resolved_value) = self.run_value(element.raw_value, element_id) {
            self.set_element_value(element_id, resolved_value, element.dependency_count == 0);
        }

        if element.dependency_count > 0 {
            return;
        }

        element.resolved = true;
        let module = self.get_module_mut(element_id.module);
        module.unresolved_count -= 1;
        if let ElementKey::Name(name) = element.key {
            log::error!("run element): {}", &*self.id2str(name));
        } else {
            log::error!("run element): {:?}", element_id.in_module);
        }
        for dependant in mem::take(&mut element.dependants) {
            self.resolve_element(dependant.element_id);
        }
        if let Some(remote_id) = &element.remote_id {
            let remote = self.get_element_remote(RemoteElementId {
                in_module: *remote_id,
                module: element_id.module,
            });
            remote.deref().cell.store(ElementRemoteCell {
                value: element.resolved_value,
                resolved: true,
            });
        }
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn run_value(&mut self, value: Value, element_id: ElementId) -> Option<TypedValue> {
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
            Value::Map(scope_id) => TypedValue {
                value: Value::Map(scope_id),
                r#type: Value::MapTy,
            },
            Value::MapTy => TypedValue {
                value: Value::MapTy,
                r#type: Value::TyTy,
            },
            Value::TyTy => TypedValue {
                value: Value::TyTy,
                r#type: Value::TyTy,
            },
            Value::Ref { name, source } => {
                let scope = self.get_element(element_id).scope.global(element_id.module);
                if let Some(ref_element_id) = self.find_element(
                    ConcurrentScopeId {
                        local: scope,
                        remote: None,
                    },
                    name,
                    true,
                ) {
                    self.depend_element_value(element_id, ref_element_id, source.upcast())?
                } else {
                    self.diagnose(
                        Location::Element(element_id),
                        Diagnostic::FailedFindElement {
                            source: source.upcast(),
                        },
                    );
                    return None;
                }
            }
            Value::Meta { name, source } => {
                let scope = self.get_element(element_id).scope.global(element_id.module);
                if let Some(ref_element_id) = self.find_element(
                    ConcurrentScopeId {
                        local: scope,
                        remote: None,
                    },
                    name,
                    true,
                ) {
                    let ConcurrentElementId::Local(ref_element_id) = ref_element_id else {
                        return None;
                    };
                    TypedValue {
                        value: Value::Element(ref_element_id),
                        r#type: Value::ElementTy,
                    }
                } else {
                    self.diagnose(
                        Location::Element(element_id),
                        Diagnostic::FailedFindElement {
                            source: source.upcast(),
                        },
                    );
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
                    Value::Map(scope_id) => {
                        log::error!("find element {}", &*self.id2str(key));
                        if let Some(find_element_id) = self.find_element(scope_id, key, false) {
                            self.depend_element_value(
                                element_id,
                                find_element_id,
                                key_source.upcast(),
                            )?
                        } else {
                            self.diagnose(
                                Location::Element(element_id),
                                Diagnostic::FailedFindElement {
                                    source: key_source.upcast(),
                                },
                            );
                            return None;
                        }
                    }
                    _ => {
                        self.diagnose(
                            Location::Element(element_id),
                            Diagnostic::CanNotFindIn {
                                source: source.upcast(),
                                value: value.value,
                            },
                        );
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
                    Value::Map(scope_id) => {
                        log::error!("find element {}", &*self.id2str(key));
                        if let Some(find_element_id) = self.find_element(scope_id, key, false) {
                            let ConcurrentElementId::Local(find_element_id) = find_element_id
                            else {
                                return None;
                            };
                            TypedValue {
                                value: Value::Element(find_element_id),
                                r#type: Value::ElementTy,
                            }
                        } else {
                            self.diagnose(
                                Location::Element(element_id),
                                Diagnostic::FailedFindElement {
                                    source: key_source.upcast(),
                                },
                            );
                            return None;
                        }
                    }
                    _ => {
                        self.diagnose(
                            Location::Element(element_id),
                            Diagnostic::CanNotFindIn {
                                source: source.upcast(),
                                value: value.value,
                            },
                        );
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
                        Builtin::If => todo!(),
                        Builtin::Add => todo!(),
                        Builtin::Mod => match param.value {
                            Value::String(string_id) => {
                                let str = erase(self).id2str(string_id);
                                let path = self
                                    .get_source_path()
                                    .join(str.deref())
                                    .with_extension(SRC_FILE_EXTENSION);
                                let abs_path = self.get_worksapce_path().join(&path);
                                if !abs_path.exists() {
                                    let param = self.get_element(param_id);
                                    self.diagnose(
                                        Location::Element(element_id),
                                        Diagnostic::PathError {
                                            source: param
                                                .authored
                                                .as_ref()
                                                .unwrap()
                                                .value_source
                                                .upcast(),
                                        },
                                    );
                                    return None;
                                }
                                let module_id = self.depend_module(path, element_id)?;
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
                                    value: Value::Map(ConcurrentScopeId {
                                        local: scope.local_id.global(module_id),
                                        remote: Some(scope_id),
                                    }),
                                    r#type: Value::MapTy,
                                }
                            }
                            _ => todo!(),
                        },
                        Builtin::Diagnose => match param.value {
                            Value::Map(scope_id) => {
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
                                let Value::Element(element) = self
                                    .depend_element_value(
                                        element_id,
                                        self.find_element(scope_id, source_key, false)?,
                                        source.upcast(),
                                    )?
                                    .value
                                else {
                                    return None;
                                };
                                if on != 0 {
                                    self.diagnose(
                                        Location::Element(element_id),
                                        Diagnostic::Custom {
                                            source: self
                                                .get_element(element)
                                                .authored
                                                .as_ref()
                                                .unwrap()
                                                .key_source
                                                .unwrap()
                                                .upcast(),
                                            text,
                                        },
                                    );
                                }
                                TypedValue {
                                    value: Value::Int(1),
                                    r#type: Value::IntTy,
                                }
                            }
                            _ => return None,
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
    /// # Panic
    /// - when concurrent, element is not in local thread.
    fn set_element_value(&mut self, element_id: ElementId, value: TypedValue, resolved: bool);
    /// # Panic
    /// when concurrent, element is not in local thread.
    fn depend_module_raw(&mut self, path: PathBuf, element_id: ElementId) -> Option<ModuleId>;
    /// # Panic
    /// when concurrent, element is not in local thread.
    fn depend_module(&mut self, path: PathBuf, element_id: ElementId) -> Option<ModuleId> {
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
        dependant_id: ElementId,
        dependency_id: ConcurrentElementId,
        source: UntypedNode<'static>,
        local: bool,
    );
    /// # Panic
    /// - when concurrent, dependant is not in local thread.
    /// - when not concurrent, dependency use remote id.
    fn depend_element_value(
        &mut self,
        dependant_id: ElementId,
        dependency_id: ConcurrentElementId,
        source: UntypedNode<'static>,
    ) -> Option<TypedValue> {
        let value = self.get_element_value(dependency_id);
        if value.is_none() {
            self.depend_element(dependant_id, dependency_id, source, true);
        }
        value
    }
    /// # Panic
    /// - when concurrent, any element is not in local thread.
    fn depend_child_element_value(
        &mut self,
        dependant_id: ElementId,
        dependency_id: ElementId,
    ) -> Option<TypedValue> {
        let dependency = self.get_element(dependency_id);
        let source = dependency.authored.as_ref().unwrap().value_source.upcast();
        self.depend_element_value(
            dependant_id,
            ConcurrentElementId::Local(dependency_id),
            source,
        )
    }
    /// # Panic
    /// element is not in threads
    fn resolve_element(&mut self, id: ElementId);
}

pub trait InterpreterLike {
    fn is_module_local(&self, module: ModuleId) -> bool;
    fn get_worksapce_path(&self) -> &Path;
    fn get_source_path(&self) -> &Path;
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
    fn get_thread_remote_of(&self, module: ModuleId) -> Option<&ThreadRemote>;
    /// # Panic
    /// when concurrent, module is in remote thread.
    fn get_module(&self, id: ModuleId) -> &Module;
    fn get_module_remote(&self, id: ModuleId) -> &ModuleRemote;
    /// # Returns
    /// - `None` if element is not resolved.
    /// # Panic
    /// when not concurrent, element id is remote

    /// # Panic
    /// when concurrent, element is in remote thread.
    fn get_element(&self, id: ElementId) -> &Element {
        self.get_module(id.module).elements.get(id.in_module)
    }
    fn get_element_remote(&self, id: RemoteElementId) -> impl Deref<Target = ElementRemote> {
        self.get_module_remote(id.module)
            .elements
            .get_concurrent(id.in_module)
    }
    fn get_element_value(&self, id: ConcurrentElementId) -> Option<TypedValue>;
    fn find_element_raw(
        &self,
        scope: ConcurrentScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<ConcurrentElementId>;
    fn find_element(
        &self,
        scope: ConcurrentScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<ConcurrentElementId> {
        if let Some(raw) = self.find_element_raw(scope, key, include_super) {
            Some(raw)
        } else {
            let builtin_module = self.get_builtin_module();
            let scope = self.get_scope(
                self.get_module(builtin_module)
                    .root_scope
                    .unwrap()
                    .global(builtin_module),
            );
            if let Some(id) = scope.elements.get(&key) {
                Some(ConcurrentElementId::Local(id.global(builtin_module)))
            } else {
                None
            }
        }
    }
    /// # Panic
    /// when concurrent, scope is in remote thread.
    fn find_element_local_raw(
        &self,
        scope_id: ScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<ElementId> {
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
    fn find_element_remote_raw(
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
    /// # Panic
    /// when concurrent, scope is in remote thread.
    fn get_scope(&self, id: ScopeId) -> &Scope {
        self.get_module(id.module).scopes.get(id.in_module)
    }
    fn get_scope_remote(&self, id: RemoteScopeId) -> impl Deref<Target = ScopeRemote> {
        self.get_module_remote(id.module)
            .scopes
            .get_concurrent(id.in_module)
    }
}

impl InterpreterLike for Interpreter {
    fn id2str(&self, id: StringId) -> impl Deref<Target = str> {
        self.strings.resolve(id)
    }
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote {
        &self.concurrent.threads.get(id).remote
    }
    fn get_thread_remote_of(&self, module: ModuleId) -> Option<&ThreadRemote> {
        if let Some(id) = self.concurrent.module2thread.get(module).copied() {
            Some(self.get_thread_remote(id))
        } else {
            None
        }
    }
    fn get_module(&self, id: ModuleId) -> &Module {
        unsafe { &self.modules[id].local.as_ref_unchecked() }
    }
    fn get_module_remote(&self, id: ModuleId) -> &ModuleRemote {
        &self.modules[id].remote
    }
    fn get_file(&self, id: FileId) -> &File {
        &self.files[id]
    }
    fn get_element_value(&self, id: ConcurrentElementId) -> Option<TypedValue> {
        match id {
            ConcurrentElementId::Local(local_element_id) => {
                let dependency = self.get_element(local_element_id);
                if dependency.resolved {
                    Some(dependency.resolved_value)
                } else {
                    None
                }
            }
            ConcurrentElementId::Remote(remote_element_id) => unimplemented!(),
        }
    }

    fn find_element_raw(
        &self,
        scope_id: ConcurrentScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<ConcurrentElementId> {
        Some(ConcurrentElementId::Local(self.find_element_local_raw(
            scope_id.local,
            key,
            include_super,
        )?))
    }

    fn get_worksapce_path(&self) -> &Path {
        &self.workspace_path
    }

    fn get_source_path(&self) -> &Path {
        Path::new(SRC_PATH)
    }

    fn is_module_local(&self, module: ModuleId) -> bool {
        true
    }

    fn find_file(&self, path: impl AsRef<Path>) -> Option<FileId> {
        self.path2file.get(path.as_ref()).copied()
    }

    fn get_builtin_module(&self) -> ModuleId {
        self.builtin_module.unwrap()
    }
}

impl<'a, IP: Deref<Target = Interpreter>> InterpreterLike for ThreadedInterpreter<'a, IP> {
    fn id2str(&self, id: StringId) -> impl Deref<Target = str> {
        self.interpreter.concurrent.strings.resolve(id)
    }
    fn get_thread_remote(&self, id: ThreadId) -> &ThreadRemote {
        self.interpreter.get_thread_remote(id)
    }
    fn get_thread_remote_of(&self, module: ModuleId) -> Option<&ThreadRemote> {
        self.interpreter.get_thread_remote_of(module)
    }
    fn get_module(&self, id: ModuleId) -> &Module {
        assert!(!self.is_module_remote(id));
        unsafe { self.interpreter.modules[id].local.as_ref_unchecked() }
    }
    fn get_module_remote(&self, id: ModuleId) -> &ModuleRemote {
        &self.interpreter.modules[id].remote
    }
    fn get_file(&self, id: FileId) -> &File {
        &self.interpreter.files[id]
    }
    fn get_element_value(&self, dependency_id: ConcurrentElementId) -> Option<TypedValue> {
        match dependency_id {
            ConcurrentElementId::Local(local_element_id) => {
                let dependency = self.get_element(local_element_id);
                if dependency.resolved {
                    Some(dependency.resolved_value)
                } else {
                    None
                }
            }
            ConcurrentElementId::Remote(remote_element_id) => {
                let dependency = self.get_element_remote(remote_element_id);
                let r#mut = dependency.cell.load();
                if r#mut.resolved {
                    Some(r#mut.value)
                } else {
                    None
                }
            }
        }
    }

    fn find_element_raw(
        &self,
        scope_id: ConcurrentScopeId,
        key: StringId,
        include_super: bool,
    ) -> Option<ConcurrentElementId> {
        if self.is_module_local(scope_id.get_module()) {
            Some(ConcurrentElementId::Local(self.find_element_local_raw(
                scope_id.local,
                key,
                include_super,
            )?))
        } else {
            Some(ConcurrentElementId::Remote(self.find_element_remote_raw(
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

    fn get_module_mut(&mut self, id: ModuleId) -> &mut Module {
        self.modules[id].local.get_mut()
    }

    fn depend_element(
        &mut self,
        dependant_id: ElementId,
        dependency_id: ConcurrentElementId,
        source: UntypedNode<'static>,
        local: bool,
    ) {
        if local {
            let dependant = erase_mut(self.get_element_mut(dependant_id));
            dependant.dependency_count += 1;
        }
        match dependency_id {
            ConcurrentElementId::Local(local_element_id) => {
                let dependency = erase_mut(self.get_element_mut(local_element_id));
                dependency.dependants.push(Dependant {
                    element_id: dependant_id,
                    source,
                });
            }
            _ => unimplemented!(),
        }
    }

    fn resolve_element(&mut self, id: ElementId) {
        let dependant = self.get_element_mut(id);
        dependant.dependency_count -= 1;
        self.run_element(id)
    }

    fn set_element_value(&mut self, element_id: ElementId, value: TypedValue, resolved: bool) {
        let element = self.get_element_mut(element_id);
        element.resolved_value = value;
        element.resolved = resolved;
    }

    fn depend_module_raw(&mut self, path: PathBuf, element_id: ElementId) -> Option<ModuleId> {
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

    fn get_module_mut(&mut self, id: ModuleId) -> &mut Module {
        assert!(self.is_module_local(id));
        unsafe { self.interpreter.modules[id].local.as_mut_unchecked() }
    }

    fn depend_element(
        &mut self,
        dependant_id: ElementId,
        dependency_id: ConcurrentElementId,
        source: UntypedNode<'static>,
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
            ConcurrentElementId::Local(local_element_id) => {
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
                        source,
                    });
                } else {
                    if let Some(thread) = self.get_thread_remote_of(local_element_id.module) {
                        thread.channel.push(Signal::Depend(Depend {
                            dependant: dependant_id,
                            dependency: local_element_id,
                            source,
                        }));
                        self.increase_workload();
                    }
                }
            }
            ConcurrentElementId::Remote(remote_element_id) => {
                let dependency = erase(self).get_element_remote(remote_element_id);
                if let Some(thread) = erase(self).get_thread_remote_of(remote_element_id.module) {
                    thread.channel.push(Signal::Depend(Depend {
                        dependant: dependant_id,
                        dependency: dependency.deref().local_id.global(remote_element_id.module),
                        source,
                    }));
                    self.increase_workload();
                }
            }
        }
    }

    fn resolve_element(&mut self, id: ElementId) {
        let thread = self
            .interpreter
            .concurrent
            .module2thread
            .get(id.module)
            .copied()
            .unwrap();

        if thread == self.thread {
            let dependant = self.get_element_mut(id);
            dependant.dependency_count -= 1;
            self.run_element(id)
        } else {
            let thread = self.get_thread_remote(thread);
            thread.channel.push(Signal::Resolve(id));
            self.increase_workload();
        }
    }

    fn set_element_value(&mut self, element_id: ElementId, value: TypedValue, resolved: bool) {
        let element = self.get_element_mut(element_id);
        element.resolved_value = value;
        element.resolved = resolved;
        if let Some(remote_id) = element.remote_id.as_ref().copied() {
            let element = self.get_element_remote(RemoteElementId {
                in_module: remote_id,
                module: element_id.module,
            });
            element.cell.store(ElementRemoteCell {
                value,
                resolved: resolved,
            });
        }
    }

    fn depend_module_raw(&mut self, path: PathBuf, element_id: ElementId) -> Option<ModuleId> {
        let element = self.get_element_mut(element_id);
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
