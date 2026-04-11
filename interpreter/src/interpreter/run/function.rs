use std::{collections::HashMap, mem};

use crate::{
    interpreter::{
        Id, Managed as _, Owner,
        element::{Element, ElementAuthored, ElementKey},
        expr::{self, Expr, HasRef as _},
        function::{
            Function, FunctionBody, FunctionElement, FunctionElementAuthored, FunctionFunction,
            FunctionScope, FunctionSet, Param,
        },
        run::{self, Runner},
        scope::Scope,
        set::Set,
        value::{self, ValueStorage},
    },
    utils::{erase, erase_mut},
};

pub struct CallContext<'a, IP> {
    ip: &'a mut IP,
    body: &'a FunctionBody,
    element_map: Vec<Option<Id<Element>>>,
    scope_map: Vec<Option<Id<Scope>>>,
    set_map: Vec<Option<Id<Set>>>,
    function_map: Vec<Option<Id<Function>>>,
    mapped_param: Id<Element>,
    owner: Owner,
    param: Id<Param>,
}

impl<'a, IP: run::InterpreterLikeMut> CallContext<'a, IP> {
    fn new<'b: 'a>(
        ip: &'b mut IP,
        body: &'b FunctionBody,
        mapped_param: Id<Element>,
        param: Id<Param>,
        owner: Owner,
    ) -> Self {
        Self {
            ip,
            body,
            element_map: vec![None; body.elements.len()],
            scope_map: vec![None; body.scopes.len()],
            set_map: vec![None; body.sets.len()],
            function_map: vec![None; body.functions.len()],
            mapped_param,
            owner,
            param,
        }
    }
    pub fn run(
        ctx: &mut super::RunExprContext<'_, IP>,
        function: value::Function,
        param: Id<Element>,
    ) -> Option<ValueStorage> {
        let function = erase(ctx.ip).get(function.0);
        let body = ctx
            .ip
            .depend_child_element(ctx.element.get_id(), function.body)?
            .extract_as_function_body()
            .0;
        let body = erase(ctx).ip.get(body);
        let mut call_ctx = CallContext::new(ctx.ip, body, param, function.param, ctx.element.owner);
        Some(ValueStorage::Scope(value::Scope(
            call_ctx.run_scope(body.root_scope.unwrap()),
        )))
    }
    fn run_set(&mut self, id: Id<Set>) -> Id<Set> {
        if let Some(id) = self.set_map.get(id.to_idx()).copied().flatten() {
            return id;
        }
        let set = erase(self).body.sets.get(id);
        let mapped_set = unsafe {
            erase_mut(self).ip.add_mut(Set {
                elements: Default::default(),
                owner: self.owner,
            })
        };
        for element_id in set.elements.iter().copied() {
            mapped_set
                .elements
                .push(self.run_element(element_id, self.owner));
        }
        self.set_map[id.to_idx()] = Some(mapped_set.get_id());
        mapped_set.get_id()
    }
    fn run_scope(&mut self, scope_id: Id<Scope>) -> Id<Scope> {
        if let Some(id) = self.scope_map.get(scope_id.to_idx()).copied().flatten() {
            return id;
        }
        let mapped_scope = unsafe { erase_mut(self).ip.add_scope(None, self.owner, None) };
        let mapped_scope_id = mapped_scope.get_id();
        let scope = self.body.scopes.get(scope_id);
        for element_id in scope.elements.iter().chain(scope.effects.iter()).copied() {
            let mapped_element_id = self.run_element(element_id, self.owner);
            let element = self.body.elements.get(element_id);
            match element.key {
                ElementKey::Name(name) => {
                    mapped_scope.elements.insert(name, mapped_element_id);
                }
                ElementKey::Effect => {
                    mapped_scope.effects.push(mapped_element_id);
                }
                _ => (),
            }
        }
        self.scope_map[scope_id.to_idx()] = Some(mapped_scope_id);
        mapped_scope_id
    }
    fn run_element(&mut self, id: Id<Element>, owner: Owner) -> Id<Element> {
        if let Some(id) = self.element_map.get(id.to_idx()).copied().flatten() {
            return id;
        }
        let function_element = self.body.elements.get(id);
        let authored = match function_element.authored {
            FunctionElementAuthored::Expr(expr) => ElementAuthored::Expr({
                let mut expr = expr.clone();
                expr.map_ref(|id| self.run_element(id, owner));
                expr
            }),
            FunctionElementAuthored::MappedValue(value) => match value {
                ValueStorage::Set(value::Set(id)) => {
                    ElementAuthored::Value(ValueStorage::Set(value::Set(self.run_set(id))))
                }
                ValueStorage::Scope(value::Scope(id)) => {
                    ElementAuthored::Value(ValueStorage::Scope(value::Scope(self.run_scope(id))))
                }
                ValueStorage::Function(value::Function(id)) => ElementAuthored::Value(
                    ValueStorage::Function(value::Function(self.run_function(id))),
                ),
                ValueStorage::Element(value::Element(id)) => ElementAuthored::Value(
                    ValueStorage::Element(value::Element(self.run_element(id, owner))),
                ),
                _ => unreachable!(),
            },
            FunctionElementAuthored::Value(value) => match value {
                ValueStorage::Param(param) => {
                    if param.0 == self.param {
                        Some(ElementAuthored::Expr(Expr::Ref(expr::Ref {
                            element: self.mapped_param,
                        })))
                    } else {
                        None
                    }
                }
                _ => None,
            }
            .unwrap_or(ElementAuthored::Value(value)),
            FunctionElementAuthored::Capture(id) => {
                ElementAuthored::Expr(Expr::Ref(expr::Ref { element: id }))
            }
        };
        let mapped_id = self
            .ip
            .add_element(function_element.key, self.owner, Some(authored))
            .get_id();
        self.element_map[id.to_idx()] = Some(mapped_id);
        mapped_id
    }
    fn run_function(&mut self, id: Id<Function>) -> Id<Function> {
        if let Some(id) = self.function_map.get(id.to_idx()).copied().flatten() {
            return id;
        }
        let function = self.body.functions.get(id);
        let mapped_funcion = erase_mut(self)
            .ip
            .add_function(self.owner, Id::DUMMY, function.param);
        let scope = {
            let parent_owner = self.owner;
            self.owner = Owner::Function(mapped_funcion.get_id());
            let scope = self.run_scope(function.scope);
            self.owner = parent_owner;
            scope
        };
        mapped_funcion.scope = scope;
        self.function_map[id.to_idx()] = Some(mapped_funcion.get_id());
        mapped_funcion.get_id()
    }
}

pub struct BodyContext<'a, IP: run::InterpreterLikeMut> {
    ip: &'a mut IP,
    function: &'a Function,
    runner: &'a mut BodyRunner,
    element: Id<Element>,
}

#[derive(Debug)]
pub struct BodyRunner {
    body: FunctionBody,
    element_map: HashMap<Id<Element>, Id<Element>>,
    scope_map: HashMap<Id<Scope>, Id<Scope>>,
    set_map: HashMap<Id<Set>, Id<Set>>,
    function_map: HashMap<Id<Function>, Id<Function>>,
    elements: Vec<(Id<Element>, Id<Element>)>,
    functions: Vec<(Id<Function>, Id<Function>)>,
    owner: Owner,
}

impl<'a, 'b: 'a, IP: run::InterpreterLikeMut> BodyContext<'a, IP> {
    pub fn run(ctx: &'a mut super::RunExprContext<'b, IP>) -> Option<ValueStorage> {
        let function_body = ctx.expr.extract_as_function_body();
        let function = erase(ctx).ip.get(function_body.function);
        if ctx.runner.is_none() {
            *ctx.runner = Some(Runner::FunctionBody(Box::new(BodyRunner {
                element_map: Default::default(),
                scope_map: Default::default(),
                set_map: Default::default(),
                function_map: Default::default(),
                elements: Default::default(),
                functions: Default::default(),
                owner: Owner::Function(function.get_id()),
                body: FunctionBody::new(),
            })));
        }
        let runner = ctx.runner.as_mut().unwrap().extract_as_function_body_mut();
        let mut body_ctx = BodyContext {
            ip: ctx.ip,
            function,
            runner,
            element: ctx.element.get_id(),
        };
        if body_ctx.runner.body.root_scope.is_none() {
            body_ctx.runner.body.root_scope = Some(body_ctx.map_scope(function.scope).unwrap());
        }
        for (element, mapped_element) in mem::take(&mut body_ctx.runner.elements) {
            body_ctx.map_element(element, Some(mapped_element));
        }
        while body_ctx.runner.elements.is_empty()
            && let Some((function, mapped_function)) = body_ctx.runner.functions.pop()
        {
            body_ctx.map_function(function, mapped_function);
        }
        if body_ctx.runner.elements.is_empty() {
            let runner = ctx.runner.take().unwrap().extract_into_function_body();
            let body = unsafe { ctx.ip.add(runner.body, ctx.module_id).get_id() };
            Some(ValueStorage::FunctionBody(value::FunctionBody(body)))
        } else {
            None
        }
    }
    fn map_set(&mut self, id: Id<Set>) -> Option<Id<Set>> {
        let vacant_entry = match erase_mut(self).runner.set_map.entry(id) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                return Some(*occupied_entry.get());
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => vacant_entry,
        };
        let set = erase(self).ip.get(id);
        if set.owner != self.runner.owner {
            return None;
        }
        let mapped_id = self.runner.body.sets.insert(FunctionSet::default());
        vacant_entry.insert(mapped_id);

        let mut mapped = FunctionSet::default();
        for element_id in set.elements.iter().copied() {
            mapped.elements.push(self.map_element(element_id, None));
        }

        *self.runner.body.sets.get_mut(mapped_id) = mapped;

        self.runner.set_map.insert(id, mapped_id);
        Some(mapped_id)
    }
    fn map_scope(&mut self, scope_id: Id<Scope>) -> Option<Id<Scope>> {
        let vacant_entry = match erase_mut(self).runner.scope_map.entry(scope_id) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                return Some(*occupied_entry.get());
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => vacant_entry,
        };
        let scope = erase(self).ip.get(scope_id);
        if scope.owner != self.runner.owner {
            return None;
        }
        let mapped_id = self.runner.body.scopes.insert(FunctionScope::DUMMY);
        vacant_entry.insert(mapped_id);

        let mut elements = vec![];
        let mut effects = vec![];
        for element in scope.elements.values().copied() {
            elements.push(self.map_element(element, None));
        }
        for element in scope.effects.iter().copied() {
            effects.push(self.map_element(element, None));
        }
        let function_scope = FunctionScope { elements, effects };

        *self.runner.body.scopes.get_mut(mapped_id) = function_scope;

        self.runner.scope_map.insert(scope_id, mapped_id);
        Some(mapped_id)
    }
    fn map_function_dummy(&mut self, id: Id<Function>) -> Option<Id<Function>> {
        let vacant_entry = match erase_mut(self).runner.function_map.entry(id) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                return Some(*occupied_entry.get());
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => vacant_entry,
        };
        let obj = erase(self).ip.get(id);
        if obj.owner != self.runner.owner {
            return None;
        }
        let mapped_id = self.runner.body.functions.insert(FunctionFunction::DUMMY);
        vacant_entry.insert(mapped_id);
        self.runner.function_map.insert(id, mapped_id);
        self.runner.functions.push((id, mapped_id));
        Some(mapped_id)
    }
    fn map_function(&mut self, id: Id<Function>, mapped_id: Id<Function>) {
        let function = self.ip.get(id);
        self.runner.owner = Owner::Function(function.get_id());
        let param = function.param;
        let scope = self.map_scope(function.scope).unwrap();
        let mapped_function = self.runner.body.functions.get_mut(mapped_id);
        mapped_function.scope = scope;
        mapped_function.param = param;
    }
    fn map_element(&mut self, id: Id<Element>, mapped_id: Option<Id<Element>>) -> Id<Element> {
        let mapped_id = if let Some(mapped_id) = mapped_id {
            mapped_id
        } else {
            let vacant_entry = match self.runner.element_map.entry(id) {
                std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    return *occupied_entry.get();
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => vacant_entry,
            };

            let mapped_id = self.runner.body.elements.insert(FunctionElement::DUMMY);
            vacant_entry.insert(mapped_id);
            mapped_id
        };

        let function_element = FunctionElement {
            authored: {
                let Some(value) = self.ip.depend_child_element(self.element, id) else {
                    self.runner.elements.push((id, mapped_id));
                    return mapped_id;
                };
                let element_local = unsafe { self.ip.get_local(id) };
                match value {
                    ValueStorage::Param(param) => {
                        let param = self.ip.get(param.0);
                        let param_element = self.ip.get(param.element);
                        if param_element.owner == Owner::Function(self.function.get_id()) {
                            if let Some(mut expr) = element_local.expr.clone() {
                                expr.map_ref(|x| self.map_element(x, None));
                                FunctionElementAuthored::Expr(expr)
                            } else {
                                FunctionElementAuthored::Value(ValueStorage::Param(value::Param(
                                    param.get_id(),
                                )))
                            }
                        } else {
                            FunctionElementAuthored::Capture(param_element.get_id())
                        }
                    }
                    ValueStorage::Scope(value::Scope(id)) => {
                        if let Some(id) = self.map_scope(id) {
                            FunctionElementAuthored::MappedValue(ValueStorage::Scope(value::Scope(
                                id,
                            )))
                        } else {
                            FunctionElementAuthored::Value(ValueStorage::Scope(value::Scope(id)))
                        }
                    }
                    ValueStorage::Element(value::Element(id)) => {
                        let element = self.ip.get(id);
                        if element.owner == Owner::Function(self.function.get_id()) {
                            FunctionElementAuthored::MappedValue(ValueStorage::Element(
                                value::Element(self.map_element(id, None)),
                            ))
                        } else {
                            FunctionElementAuthored::Value(ValueStorage::Element(value::Element(
                                id,
                            )))
                        }
                    }
                    ValueStorage::Set(value::Set(id)) => {
                        if let Some(id) = self.map_set(id) {
                            FunctionElementAuthored::MappedValue(ValueStorage::Set(value::Set(id)))
                        } else {
                            FunctionElementAuthored::Value(ValueStorage::Set(value::Set(id)))
                        }
                    }
                    ValueStorage::Function(value::Function(id)) => {
                        if let Some(id) = self.map_function_dummy(id) {
                            FunctionElementAuthored::MappedValue(ValueStorage::Function(
                                value::Function(id),
                            ))
                        } else {
                            FunctionElementAuthored::Value(ValueStorage::Function(value::Function(
                                id,
                            )))
                        }
                    }
                    _ => FunctionElementAuthored::Value(value),
                }
            },
            key: self.ip.get(id).key,
        };

        *self.runner.body.elements.get_mut(mapped_id) = function_element;

        mapped_id
    }
}
