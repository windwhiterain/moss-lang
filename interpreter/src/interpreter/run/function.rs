use std::collections::HashMap;

use hashbrown::HashSet;

use crate::{
    interpreter::{
        Id, InterpreterLikeMut, Managed as _,
        element::{Element, ElementAuthored},
        expr::HasRef as _,
        function::{
            Function, FunctionElement, FunctionElementAuthored, FunctionOptimized, FunctionScope,
            OPTIMIZED_PARAM,
        },
        module::ModuleId,
        scope::Scope,
        value::{self, Value},
    },
    utils::{contexted::WithContext, erase, erase_mut},
};

pub struct CallContext<'a, IP> {
    interpreter: &'a mut IP,
    optimized: &'a FunctionOptimized,
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
        let _ = ctx
            .ip
            .depend_child_element(ctx.element_id, function.complete)?;
        let optimized = unsafe { function.optimized.as_ref_unchecked() };
        let mut ctx = CallContext {
            interpreter: ctx.ip,
            optimized,
            module_id: ctx.module_id,
            element_map: Default::default(),
            scope_map: Default::default(),
            param,
        };
        Some(Value::Scope(value::Scope(
            ctx.instantiate_scope(optimized.root_scope.unwrap()),
        )))
    }
    fn instantiate_scope(&mut self, scope_id: Id<Scope>) -> Id<Scope> {
        if let Some(id) = self.scope_map.get(scope_id.to_idx()).copied().flatten() {
            return id;
        }
        let mapped_scope = unsafe {
            erase_mut(self)
                .interpreter
                .add_scope(None, None, self.module_id)
        };
        let mapped_scope_id = mapped_scope.get_id();
        let function_scope = self.optimized.scopes.get(scope_id);
        for element in function_scope.elements.iter().copied() {
            self.instantiate_element(mapped_scope, element);
        }
        if self.scope_map.len() <= scope_id.to_idx() {
            self.scope_map
                .resize(scope_id.to_idx() + 1, Default::default());
        }
        self.scope_map[scope_id.to_idx()] = Some(mapped_scope_id);
        mapped_scope_id
    }
    fn instantiate_element(&mut self, scope: &mut Scope, id: Id<Element>) -> Id<Element> {
        if id == OPTIMIZED_PARAM {
            return self.param;
        }
        if let Some(id) = self.element_map.get(id.to_idx()).copied().flatten() {
            return id;
        }
        let function_element = self.optimized.elements.get(id);
        let authored = match &function_element.authored {
            FunctionElementAuthored::Expr(expr) => ElementAuthored::Expr({
                let mut expr = expr.clone();
                expr.map_ref(|id| self.instantiate_element(scope, id));
                expr
            }),
            FunctionElementAuthored::Value(value) => {
                let value = match *value {
                    Value::Scope(value::Scope(id)) => {
                        Value::Scope(value::Scope(self.instantiate_scope(id)))
                    }
                    _ => *value,
                };
                ElementAuthored::Value(value)
            }
        };
        let new_id = self
            .interpreter
            .add_element(function_element.key, scope, Some(authored))
            .unwrap();
        if self.element_map.len() <= id.to_idx() {
            self.element_map.resize(id.to_idx() + 1, Default::default());
        }
        self.element_map[id.to_idx()] = Some(new_id);
        new_id
    }
}

pub struct OptimizeContext<'a, IP: InterpreterLikeMut> {
    ip: &'a mut IP,
    element_id: Id<Element>,
    function: &'a Function,
    optimized: &'a mut FunctionOptimized,
    resolved_scopes: HashSet<Id<Scope>>,
    element_map: HashMap<Id<Element>, Id<Element>>,
    scope_map: HashMap<Id<Scope>, Id<Scope>>,
}

impl<'a, 'b: 'a, IP: InterpreterLikeMut> OptimizeContext<'a, IP> {
    pub fn run(ctx: &'a mut super::Context<'b, IP>) -> Option<Value> {
        let function_optimize = ctx.expr.extract_as_function_optimize();
        let function = erase(ctx).ip.get(function_optimize.function);
        let optimized = unsafe { function.optimized.as_mut_unchecked() };
        let mut ctx = OptimizeContext {
            ip: ctx.ip,
            element_id: ctx.element_id,
            function,
            optimized,
            resolved_scopes: Default::default(),
            element_map: Default::default(),
            scope_map: Default::default(),
        };
        ctx.depend_scope(function.scope)?;
        log::error!("{}", value::Scope(function.scope).with_ctx(ctx.ip));
        ctx.optimized.root_scope = Some(ctx.map_scope(function.scope));
        Some(Value::Trivial(value::Trivial))
    }
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

        let mapped_id = self.optimized.scopes.insert(FunctionScope::DUMMY);
        vacant_entry.insert(mapped_id);

        let scope = erase(self).ip.get(scope_id);
        let mut elements = vec![];
        for element in scope.elements.values().copied() {
            elements.push(self.map_element(element));
        }
        let function_scope = FunctionScope { elements };

        *self.optimized.scopes.get_mut(mapped_id) = function_scope;

        self.scope_map.insert(scope_id, mapped_id);
        mapped_id
    }
    fn map_element(&mut self, element_id: Id<Element>) -> Id<Element> {
        if element_id == self.function.r#in {
            return OPTIMIZED_PARAM;
        }
        let vacant_entry = match self.element_map.entry(element_id) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                return *occupied_entry.get();
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => vacant_entry,
        };

        let mapped_id = self.optimized.elements.insert(FunctionElement::DUMMY);
        vacant_entry.insert(mapped_id);

        let function_element = FunctionElement {
            authored: {
                let element_local = unsafe { self.ip.get_local(element_id) };
                let value = element_local.value.unwrap();
                match value {
                    Value::Param(param) => {
                        if self.ip.get(param.0).function == self.function.get_id() {
                            FunctionElementAuthored::Expr({
                                let mut expr = element_local.expr.clone().unwrap();
                                expr.map_ref(|x| self.map_element(x));
                                expr
                            })
                        } else {
                            FunctionElementAuthored::Value(value)
                        }
                    }
                    Value::Scope(value::Scope(id)) => {
                        let id = self.map_scope(id);
                        FunctionElementAuthored::Value(Value::Scope(value::Scope(id)))
                    }
                    _ => FunctionElementAuthored::Value(value),
                }
            },
            key: self.ip.get(element_id).key,
        };

        *self.optimized.elements.get_mut(mapped_id) = function_element;

        mapped_id
    }
}
