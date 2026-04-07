use std::collections::HashMap;

use crate::{
    interpreter::{
        Id, InterpreterLikeMut, Managed as _,
        element::{Element, ElementAuthored, ElementKey},
        expr::{self, Expr, HasRef as _},
        function::{
            Function, FunctionBody, FunctionElement, FunctionElementAuthored, FunctionFunction,
            FunctionScope, FunctionSet,
        },
        module::ModuleId,
        scope::Scope,
        set::Set,
        value::{self, ValueStorage},
    },
    utils::{erase, erase_mut},
};

pub struct CallContext<'a, IP> {
    ip: &'a mut IP,
    expr: &'a mut Expr,
    body: &'a FunctionBody,
    module_id: ModuleId,
    element_map: Vec<Option<Id<Element>>>,
    scope_map: Vec<Option<Id<Scope>>>,
    param: Id<Element>,
}

impl<'a, IP: InterpreterLikeMut> CallContext<'a, IP> {
    pub fn run(
        ctx: &mut super::Context<'a, IP>,
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
        let mut call_ctx = CallContext {
            ip: ctx.ip,
            expr: ctx.expr,
            body,
            module_id: ctx.module_id,
            element_map: Default::default(),
            scope_map: Default::default(),
            param,
        };
        *call_ctx.expr = Expr::Value(ValueStorage::Scope(value::Scope(
            call_ctx.run_scope(body.root_scope.unwrap()),
        )));
        ctx.run()
    }
    fn run_set(&mut self, set_id: Id<Set>) -> Id<Set> {
        let set = erase(self).body.sets.get(set_id);
        let mapped_set = unsafe {
            erase_mut(self).ip.add(
                Set {
                    elements: Default::default(),
                    module: Default::default(),
                },
                self.module_id,
            )
        };
        for element_id in set.elements.iter().copied() {
            mapped_set.elements.push(self.run_element(element_id));
        }
        mapped_set.get_id()
    }
    fn run_scope(&mut self, scope_id: Id<Scope>) -> Id<Scope> {
        if let Some(id) = self.scope_map.get(scope_id.to_idx()).copied().flatten() {
            return id;
        }
        let mapped_scope = unsafe { erase_mut(self).ip.add_scope(None, None, self.module_id) };
        let mapped_scope_id = mapped_scope.get_id();
        let scope = self.body.scopes.get(scope_id);
        for element_id in scope.elements.iter().chain(scope.effects.iter()).copied() {
            let mapped_element_id = self.run_element(element_id);
            if element_id != FunctionBody::PARAM_ELEMENT_ID {
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
        }
        if self.scope_map.len() <= scope_id.to_idx() {
            self.scope_map
                .resize(scope_id.to_idx() + 1, Default::default());
        }
        self.scope_map[scope_id.to_idx()] = Some(mapped_scope_id);
        mapped_scope_id
    }
    fn run_element(&mut self, id: Id<Element>) -> Id<Element> {
        if id == FunctionBody::PARAM_ELEMENT_ID {
            return self.param;
        }
        if let Some(id) = self.element_map.get(id.to_idx()).copied().flatten() {
            return id;
        }
        let function_element = self.body.elements.get(id);
        let authored = match &function_element.authored {
            FunctionElementAuthored::Expr(expr) => ElementAuthored::Expr({
                let mut expr = expr.clone();
                expr.map_ref(|id| self.run_element(id));
                expr
            }),
            FunctionElementAuthored::Value(value) => {
                let value = match *value {
                    ValueStorage::Set(value::Set(id)) => {
                        ValueStorage::Set(value::Set(self.run_set(id)))
                    }
                    ValueStorage::Scope(value::Scope(id)) => {
                        ValueStorage::Scope(value::Scope(self.run_scope(id)))
                    }
                    ValueStorage::Function(value::Function(id)) => {
                        ValueStorage::Function(value::Function(self.run_function(id)))
                    }
                    ValueStorage::Element(value::Element(id)) => {
                        ValueStorage::Element(value::Element(self.run_element(id)))
                    }
                    _ => *value,
                };
                ElementAuthored::Expr(Expr::Value(value))
            }
            FunctionElementAuthored::Capture(id) => {
                ElementAuthored::Expr(Expr::Ref(expr::Ref { element: *id }))
            }
        };
        let mapped_id = self
            .ip
            .add_element(function_element.key, self.module_id, Some(authored))
            .get_id();
        if self.element_map.len() <= id.to_idx() {
            self.element_map.resize(id.to_idx() + 1, Default::default());
        }
        self.element_map[id.to_idx()] = Some(mapped_id);
        mapped_id
    }
    fn run_function(&mut self, id: Id<Function>) -> Id<Function> {
        let function = erase(self).body.functions.get(id);
        let scope = self.run_scope(function.scope);
        let param = self.run_element(function.param);
        let mapped_funcion = erase_mut(self)
            .ip
            .add_function(self.module_id, scope, param);
        mapped_funcion.get_id()
    }
}

pub struct BodyContext<'a, IP: InterpreterLikeMut> {
    ip: &'a mut IP,
    function: &'a Function,
    body: &'a mut FunctionBody,
    element_map: HashMap<Id<Element>, Id<Element>>,
    scope_map: HashMap<Id<Scope>, Id<Scope>>,
}

impl<'a, 'b: 'a, IP: InterpreterLikeMut> BodyContext<'a, IP> {
    pub fn run(ctx: &'a mut super::Context<'b, IP>) -> Option<ValueStorage> {
        let function_body = ctx.expr.extract_as_function_body();
        let function = erase(ctx).ip.get(function_body.function);

        let body = unsafe { erase_mut(ctx).ip.add(FunctionBody::new(), ctx.module_id) };
        let mut ctx = BodyContext {
            ip: ctx.ip,
            function,
            body,
            element_map: Default::default(),
            scope_map: Default::default(),
        };
        ctx.body.root_scope = Some(ctx.map_scope(function.scope));
        Some(ValueStorage::FunctionBody(value::FunctionBody(
            ctx.body.get_id(),
        )))
    }
    fn map_set(&mut self, set_id: Id<Set>) -> Id<Set> {
        let set = erase(self).ip.get(set_id);
        let mut mapped_function = FunctionSet::default();
        for element_id in set.elements.iter().copied() {
            mapped_function.elements.push(self.map_element(element_id));
        }
        self.body.sets.insert(mapped_function)
    }
    fn map_scope(&mut self, scope_id: Id<Scope>) -> Id<Scope> {
        if let Some(mapped) = self.scope_map.get(&scope_id).copied() {
            return mapped;
        }
        let vacant_entry = match self.scope_map.entry(scope_id) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                return *occupied_entry.get();
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => vacant_entry,
        };

        let mapped_id = self.body.scopes.insert(FunctionScope::DUMMY);
        vacant_entry.insert(mapped_id);

        let scope = erase(self).ip.get(scope_id);
        let mut elements = vec![];
        let mut effects = vec![];
        for element in scope.elements.values().copied() {
            elements.push(self.map_element(element));
        }
        for element in scope.effects.iter().copied() {
            effects.push(self.map_element(element));
        }
        let function_scope = FunctionScope { elements, effects };

        *self.body.scopes.get_mut(mapped_id) = function_scope;

        self.scope_map.insert(scope_id, mapped_id);
        mapped_id
    }
    fn map_element(&mut self, element_id: Id<Element>) -> Id<Element> {
        if element_id == self.function.param {
            return FunctionBody::PARAM_ELEMENT_ID;
        }
        let vacant_entry = match self.element_map.entry(element_id) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                return *occupied_entry.get();
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => vacant_entry,
        };

        let mapped_id = self.body.elements.insert(FunctionElement::DUMMY);
        vacant_entry.insert(mapped_id);

        let function_element = FunctionElement {
            authored: {
                let element_local = unsafe { self.ip.get_local(element_id) };
                let value = element_local.value.unwrap();
                match value {
                    ValueStorage::Param(param) => {
                        let param = self.ip.get(param.0);
                        if param.function == self.function.get_id() {
                            FunctionElementAuthored::Expr({
                                let mut expr = element_local.expr.clone().unwrap();
                                expr.map_ref(|x| self.map_element(x));
                                expr
                            })
                        } else {
                            FunctionElementAuthored::Capture(param.element)
                        }
                    }
                    ValueStorage::Scope(value::Scope(id)) => {
                        let id = self.map_scope(id);
                        FunctionElementAuthored::Value(ValueStorage::Scope(value::Scope(id)))
                    }
                    ValueStorage::Function(value::Function(id)) => {
                        let id = self.map_function(id);
                        FunctionElementAuthored::Value(ValueStorage::Function(value::Function(id)))
                    }
                    ValueStorage::Element(value::Element(id)) => {
                        let element = unsafe { self.ip.get_local(id) };
                        let id = if let ValueStorage::Param(param) = element.value.unwrap() {
                            self.ip.get(param.0).element
                        } else {
                            id
                        };
                        FunctionElementAuthored::Value(ValueStorage::Element(value::Element(
                            self.map_element(id),
                        )))
                    }
                    ValueStorage::Set(value::Set(set)) => {
                        let id = self.map_set(set);
                        FunctionElementAuthored::Value(ValueStorage::Set(value::Set(id)))
                    }
                    _ => FunctionElementAuthored::Value(value),
                }
            },
            key: self.ip.get(element_id).key,
        };

        *self.body.elements.get_mut(mapped_id) = function_element;

        mapped_id
    }
    fn map_function(&mut self, function_id: Id<Function>) -> Id<Function> {
        let function = erase(self).ip.get(function_id);
        let mapped_function = FunctionFunction::new(
            self.map_scope(function.scope),
            self.map_element(function.param),
        );
        self.body.functions.insert(mapped_function)
    }
}
