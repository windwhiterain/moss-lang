use std::collections::HashMap;

use hashbrown::HashSet;

use crate::{
    interpreter::{
        Id, InterpreterLikeMut, Managed as _,
        element::{Element, ElementAuthored},
        expr::{self, Expr, HasRef as _},
        function::{
            Function, FunctionBody, FunctionElement, FunctionElementAuthored, FunctionFunction,
            FunctionScope,
        },
        module::ModuleId,
        scope::Scope,
        value::{self, Value},
    },
    utils::{contexted::WithContext, erase, erase_mut},
};

pub struct CallContext<'a, IP> {
    ip: &'a mut IP,
    captures: &'a Vec<Id<Element>>,
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
    ) -> Option<Value> {
        let function = erase(ctx.ip).get(function.0);
        let captures = unsafe { function.captures.as_ref_unchecked() };
        let body = ctx
            .ip
            .depend_child_element(ctx.element.get_id(), function.body)?
            .extract_as_function_body()
            .0;
        let body = erase(ctx).ip.get(body);
        log::error!("function_body {:#?}", body);
        let mut ctx = CallContext {
            ip: ctx.ip,
            captures,
            body,
            module_id: ctx.module_id,
            element_map: Default::default(),
            scope_map: Default::default(),
            param,
        };
        Some(Value::Scope(value::Scope(
            ctx.run_scope(body.root_scope.unwrap()),
        )))
    }
    fn run_scope(&mut self, scope_id: Id<Scope>) -> Id<Scope> {
        if let Some(id) = self.scope_map.get(scope_id.to_idx()).copied().flatten() {
            return id;
        }
        let mapped_scope = unsafe { erase_mut(self).ip.add_scope(None, None, self.module_id) };
        let mapped_scope_id = mapped_scope.get_id();
        let scope = self.body.scopes.get(scope_id);
        for element_id in scope.elements.iter().copied() {
            let mapped_element_id = self.run_element(element_id);
            if element_id != FunctionBody::PARAM_ELEMENT_ID {
                let element = self.body.elements.get(element_id);
                mapped_scope
                    .elements
                    .insert(*element.key.extract_as_name(), mapped_element_id);
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
                    Value::Scope(value::Scope(id)) => {
                        Value::Scope(value::Scope(self.run_scope(id)))
                    }
                    Value::Function(value::Function(id)) => {
                        Value::Function(value::Function(self.run_function(id)))
                    }
                    _ => *value,
                };
                ElementAuthored::Value(value)
            }
            FunctionElementAuthored::Capture(id) => {
                let element_id = self.captures[*id];
                ElementAuthored::Expr(Expr::Ref(expr::Ref { element_id }))
            }
        };
        let mapped_id = self
            .ip
            .add_element(function_element.key, self.module_id, Some(authored))
            .unwrap()
            .get_id();
        if self.element_map.len() <= id.to_idx() {
            self.element_map.resize(id.to_idx() + 1, Default::default());
        }
        self.element_map[id.to_idx()] = Some(mapped_id);
        mapped_id
    }
    fn run_function(&mut self, id: Id<Function>) -> Id<Function> {
        let function = erase(self).body.functions.get(id);
        let mapped_funcion = unsafe {
            erase_mut(self).ip.add(
                Function::new(Id::DUMMY, Id::DUMMY, ModuleId::default(), function.body),
                self.module_id,
            )
        };
        for element_id in &function.captures {
            mapped_funcion
                .captures
                .get_mut()
                .push(self.run_element(*element_id));
        }
        mapped_funcion.get_id()
    }
}

pub struct BodyDependContext<'a, IP: InterpreterLikeMut> {
    ip: &'a mut IP,
    element_id: Id<Element>,
    resolved_scopes: HashSet<Id<Scope>>,
}

impl<'a, 'b: 'a, IP: InterpreterLikeMut> BodyDependContext<'a, IP> {
    fn depend_scope(&mut self, scope_id: Id<Scope>) -> Option<()> {
        if !self.resolved_scopes.insert(scope_id) {
            return Some(());
        }
        let scope = erase(self).ip.get(scope_id);
        for element_id in scope.elements.values().copied() {
            self.depend_element(element_id)?
        }
        Some(())
    }
    fn depend_element(&mut self, element_id: Id<Element>) -> Option<()> {
        let value = self.ip.depend_child_element(self.element_id, element_id)?;
        if let Value::Scope(value::Scope(scope_id)) = value {
            self.depend_scope(scope_id)?;
        }
        Some(())
    }
}

pub struct BodyContext<'a, IP: InterpreterLikeMut> {
    ip: &'a mut IP,
    function: &'a Function,
    captures: &'a mut Vec<Id<Element>>,
    body: &'a mut FunctionBody,
    element_map: HashMap<Id<Element>, Id<Element>>,
    scope_map: HashMap<Id<Scope>, Id<Scope>>,
}

impl<'a, 'b: 'a, IP: InterpreterLikeMut> BodyContext<'a, IP> {
    pub fn run(ctx: &'a mut super::Context<'b, IP>) -> Option<Value> {
        let function_body = ctx.expr.extract_as_function_body();
        let function = erase(ctx).ip.get(function_body.function);
        {
            let mut ctx = BodyDependContext {
                ip: ctx.ip,
                element_id: ctx.element.get_id(),
                resolved_scopes: Default::default(),
            };
            ctx.depend_scope(function.scope)?;
        }
        let captures = unsafe { erase(function).captures.as_mut_unchecked() };
        let body = unsafe { erase_mut(ctx).ip.add(FunctionBody::new(), ctx.module_id) };
        let mut ctx = BodyContext {
            ip: ctx.ip,
            function,
            captures,
            body,
            element_map: Default::default(),
            scope_map: Default::default(),
        };
        ctx.body.root_scope = Some(ctx.map_scope(function.scope));
        Some(Value::FunctionBody(value::FunctionBody(ctx.body.get_id())))
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
        for element in scope.elements.values().copied() {
            elements.push(self.map_element(element));
        }
        let function_scope = FunctionScope { elements };

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
                    Value::Param(param) => {
                        let param = self.ip.get(param.0);
                        if param.function == self.function.get_id() {
                            FunctionElementAuthored::Expr({
                                let mut expr = element_local.expr.clone().unwrap();
                                expr.map_ref(|x| self.map_element(x));
                                expr
                            })
                        } else {
                            let id = self.captures.len();
                            self.captures.push(param.element);
                            FunctionElementAuthored::Capture(id)
                        }
                    }
                    Value::Scope(value::Scope(id)) => {
                        let id = self.map_scope(id);
                        FunctionElementAuthored::Value(Value::Scope(value::Scope(id)))
                    }
                    Value::Function(value::Function(id)) => {
                        let id = self.map_function(id);
                        FunctionElementAuthored::Value(Value::Function(value::Function(id)))
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
        let mut mapped_function = FunctionFunction::new(function.body);
        for element_id in unsafe { function.captures.as_ref_unchecked() }
            .iter()
            .copied()
        {
            mapped_function.captures.push(self.map_element(element_id));
        }
        self.body.functions.insert(mapped_function)
    }
}
