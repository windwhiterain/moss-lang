use std::borrow::Cow;

use type_sitter::{HasChild as _, Node as _, NodeResult};

use crate::{
    erase_struct,
    interpreter::{
        Id, InterpreterLikeBasicMut, Location, Managed, Owner,
        element::{Element, ElementAuthored, ElementKey, ElementSource},
        error::Kind,
        expr::{self, Expr},
        file::FileId,
        function::Param,
        module::Module,
        scope::Scope,
        set::Set,
        value::{self, ValueStorage},
    },
    utils::moss,
};

use crate::utils::erase;
use crate::utils::erase_mut;

struct Context<'a, IP: ?Sized> {
    pub ip: &'a mut IP,
    pub source_child: moss::ValueChild<'static>,
    pub element_id: Id<Element>,
    pub scope: &'a mut Scope,
    pub module: &'a Module,
}

enum FindSource {
    Find(moss::Find<'static>),
    MetaFind(moss::MetaFind<'static>),
    FindIn(moss::FindIn<'static>),
    MetaFindIn(moss::MetaFindIn<'static>),
}

impl<'a, IP: ?Sized + InterpreterLikeBasicMut> Context<'a, IP> {
    fn parse_call(&mut self, call: moss::Call<'static>) -> Option<Expr> {
        let func = unsafe {
            self.ip
                .grammar_error(Location::Element(self.element_id), call.func())
        }?;
        let param = unsafe {
            self.ip
                .grammar_error(Location::Element(self.element_id), call.param())
        }?;
        let func_element = self
            .ip
            .add_element(
                ElementKey::Temp,
                self.ip.get(self.element_id).owner,
                Some(ElementAuthored::Source {
                    source: ElementSource {
                        scope: self.scope.get_id(),
                        value_source: func,
                        key_source: None,
                    },
                    scope: self.scope,
                }),
            )
            .get_id();
        let param_element = self
            .ip
            .add_element(
                ElementKey::Temp,
                self.ip.get(self.element_id).owner,
                Some(ElementAuthored::Source {
                    source: ElementSource {
                        scope: self.scope.get_id(),
                        value_source: param,
                        key_source: None,
                    },
                    scope: self.scope,
                }),
            )
            .get_id();
        Some(Expr::Call(expr::Call {
            function: func_element,
            param: param_element,
        }))
    }
    fn parse_scope(&mut self, scope: moss::Scope<'static>) -> Option<Expr> {
        Some(Expr::Value(ValueStorage::Scope(value::Scope(unsafe {
            // SAFETY: element -> scope
            let source = if let Some(scope) = scope.scope_content() {
                Some(
                    self.ip
                        .grammar_error(Location::Element(self.element_id), scope)?,
                )
            } else {
                None
            };
            self.ip
                .add_scope(Some(self.scope.get_id()), self.scope.owner, source)
                .get_id()
        }))))
    }
    fn parse_find(&mut self, find: FindSource) -> Option<Expr> {
        let (target, name, meta) = unsafe {
            match find {
                FindSource::FindIn(find) => (
                    Some(
                        self.ip
                            .grammar_error(Location::Element(self.element_id), find.value())?,
                    ),
                    self.ip
                        .grammar_error(Location::Element(self.element_id), find.name())?
                        .upcast(),
                    false,
                ),
                FindSource::MetaFindIn(find) => (
                    Some(
                        self.ip
                            .grammar_error(Location::Element(self.element_id), find.value())?,
                    ),
                    self.ip
                        .grammar_error(Location::Element(self.element_id), find.name())?
                        .upcast(),
                    true,
                ),
                FindSource::Find(find) => (
                    None,
                    self.ip
                        .grammar_error(Location::Element(self.element_id), find.name())?
                        .upcast(),
                    false,
                ),
                FindSource::MetaFind(meta) => (
                    None,
                    self.ip
                        .grammar_error(Location::Element(self.element_id), meta.name())?
                        .upcast(),
                    true,
                ),
            }
        };
        let target = if let Some(target) = target {
            Some(
                self.ip
                    .add_element(
                        ElementKey::Temp,
                        self.ip.get(self.element_id).owner,
                        Some(ElementAuthored::Source {
                            source: ElementSource {
                                scope: self.scope.get_id(),
                                value_source: target,
                                key_source: None,
                            },
                            scope: self.scope,
                        }),
                    )
                    .get_id(),
            )
        } else {
            None
        };
        Some(Expr::Find(expr::Find {
            target,
            name: self.ip.get_source_str_id(&name, self.module),
            meta,
        }))
    }
    fn parse_string(&mut self, string: moss::String<'static>) -> Option<Expr> {
        let mut cursor = erase_struct!(
            self.ip
                .get_file(self.module.authored.unwrap().file)
                .tree
                .walk()
        );
        let mut value: Option<Cow<str>> = None;
        for content in string.contents(erase_mut(&mut cursor)) {
            let content = unsafe {
                erase_mut(self)
                    .ip
                    .grammar_error(Location::Element(self.element_id), content)
            }?;
            let content_value = match unsafe {
                erase_mut(self)
                    .ip
                    .grammar_error(Location::Element(self.element_id), content.child())
            }? {
                moss::StringContentChild::StringEscape(string_escape) => {
                    match erase(self).ip.get_source_str(&string_escape, self.module) {
                        "\\\"" => Some("\""),
                        "\\\\" => Some("\\"),
                        "\\n" => Some("\n"),
                        "\\t" => Some("\t"),
                        "\\r" => Some("\r"),
                        "\\{" => Some("{"),
                        "\\}" => Some("}"),
                        _ => {
                            unsafe {
                                erase_mut(self).ip.diagnose(
                                    Location::Element(self.element_id),
                                    Kind::StringEscapeError {},
                                )
                            };
                            None
                        }
                    }
                }
                moss::StringContentChild::StringRaw(string_raw) => {
                    Some(erase(self).ip.get_source_str(&string_raw, self.module))
                }
            }?;
            if let Some(value) = &mut value {
                value.to_mut().push_str(content_value);
            } else {
                value = Some(Cow::Borrowed(content_value))
            }
        }
        Some(Expr::Value(ValueStorage::String(value::String(
            self.ip
                .str2id(value.as_ref().map(|x| x.as_ref()).unwrap_or("")),
        ))))
    }
    fn parse_function(&mut self, function: moss::Function<'static>) -> Option<Expr> {
        let module_id = self.scope.get_module(self.ip);
        let (param_name, scope) = unsafe {
            let param_name = self
                .ip
                .grammar_error(Location::Element(self.element_id), function.param())?;
            let scope = self
                .ip
                .grammar_error(Location::Element(self.element_id), function.scope())?;
            (param_name, scope)
        };
        let param_name = self.ip.get_source_str_id(&param_name, self.module);

        let source = if let Some(scope) = scope.scope_content() {
            Some(unsafe {
                self.ip
                    .grammar_error(Location::Element(self.element_id), scope)
            }?)
        } else {
            None
        };

        let function = erase_mut(self)
            .ip
            .add_function(self.scope.owner, Id::DUMMY, Id::DUMMY);

        let scope = unsafe {
            // SAFETY: element -> scope
            erase_mut(self).ip.add_scope(
                Some(self.scope.get_id()),
                Owner::Function(function.get_id()),
                source,
            )
        };
        function.scope = scope.get_id();

        let param = unsafe {
            erase_mut(self).ip.add(
                Param {
                    function: function.get_id(),
                    element: Id::DUMMY,
                    r#type: None,
                },
                module_id,
            )
        };
        function.param = param.get_id();
        let param_element = erase_mut(self.ip.add_element(
            ElementKey::Name(param_name),
            Owner::Function(function.get_id()),
            Some(ElementAuthored::Value(ValueStorage::Param(value::Param(
                param.get_id(),
            )))),
        ));
        param.element = param_element.get_id();
        scope.elements.insert(param_name, param_element.get_id());

        Some(Expr::Value(ValueStorage::Function(value::Function(
            function.get_id(),
        ))))
    }
    fn parse_set(&mut self, set_source: moss::Set<'static>) -> Option<Expr> {
        let mut cursor = erase_struct!(
            self.ip
                .get_file(self.module.authored.unwrap().file)
                .tree
                .walk()
        );
        let set = erase_mut(unsafe {
            self.ip.add_mut(Set {
                elements: Default::default(),
                owner: self.scope.owner,
            })
        });

        for value in set_source.values(&mut cursor) {
            if let Some(value) = unsafe {
                self.ip
                    .grammar_error(Location::Element(self.element_id), value)
            } {
                let element = self.ip.add_element(
                    ElementKey::Temp,
                    self.ip.get(self.element_id).owner,
                    Some(ElementAuthored::Source {
                        source: ElementSource {
                            value_source: value,
                            key_source: None,
                            scope: self.scope.get_id(),
                        },
                        scope: self.scope,
                    }),
                );
                set.elements.push(element.get_id());
            }
        }
        Some(Expr::Value(ValueStorage::Set(value::Set(set.get_id()))))
    }
    fn parse(&mut self) -> Option<Expr> {
        match self.source_child {
            moss::ValueChild::Int(int) => Some(Expr::Value(ValueStorage::Int(value::Int(
                self.ip.get_source_str(&int, self.module).parse().unwrap(),
            )))),
            moss::ValueChild::String(string) => self.parse_string(string),
            moss::ValueChild::Call(call) => self.parse_call(call),
            moss::ValueChild::Scope(scope) => self.parse_scope(scope),
            moss::ValueChild::Find(find) => self.parse_find(FindSource::Find(find)),
            moss::ValueChild::MetaFind(meta) => self.parse_find(FindSource::MetaFind(meta)),
            moss::ValueChild::FindIn(find) => self.parse_find(FindSource::FindIn(find)),
            moss::ValueChild::MetaFindIn(find_meta) => {
                self.parse_find(FindSource::MetaFindIn(find_meta))
            }
            moss::ValueChild::Function(function) => self.parse_function(function),
            moss::ValueChild::Bracket(bracket) => {
                parse_value(self.ip, bracket.value(), self.element_id, self.scope)
            }
            moss::ValueChild::Set(set) => self.parse_set(set),
            moss::ValueChild::Trivial(_) => {
                Some(Expr::Value(ValueStorage::Trivial(value::Trivial)))
            }
        }
    }
}

pub fn parse_value<IP: ?Sized + InterpreterLikeBasicMut>(
    ip: &mut IP,
    source: NodeResult<'static, moss::Value<'static>>,
    element_id: Id<Element>,
    scope: &mut Scope,
) -> Option<Expr> {
    let source = unsafe { ip.grammar_error(Location::Element(element_id), source) }?;
    let source_child: moss::ValueChild =
        unsafe { ip.grammar_error(Location::Element(element_id), source.child()) }?;
    let module = erase(ip).get_module(scope.get_module(ip));
    let mut ctx = Context {
        ip,
        source_child,
        element_id,
        scope,
        module,
    };
    ctx.parse()
}
