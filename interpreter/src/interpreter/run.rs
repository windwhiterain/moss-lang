use std::{mem, sync::OnceLock};

use enum_extract_macro::EnumExtract;
use type_sitter::{Node, UntypedNode};

use crate::{
    interpreter::{
        Id, InterpreterLikeBasicMut, Location, Managed, Owner,
        diagnose::Diagnostic,
        element::{Dependant, Element},
        expr::{self, Expr},
        module::ModuleId,
        scope::Scope,
        thread::{Depend, Resolve, Signal},
        value::{self, ValueStorage},
    },
    utils::{contexted::WithContext as _, erase, erase_mut},
};

mod buitin_function;
mod function;

#[derive(Debug, EnumExtract)]
pub enum Runner {
    FunctionBody(Box<function::BodyRunner>),
}

pub struct RunExprContext<'a, IP> {
    ip: &'a mut IP,
    element: &'a Element,
    module_id: ModuleId,
    source: Option<UntypedNode<'static>>,
    expr: &'a mut Expr,
    runner: &'a mut Option<Runner>,
}

impl<'a, IP: InterpreterLikeMut> RunExprContext<'a, IP> {
    fn run_expr(ip: &'a mut IP, element_id: Id<Element>) -> Option<ValueStorage> {
        let element = erase(ip).get(element_id);
        let module_id = element.get_module(ip);
        let source = element.source.map(|x| x.value_source.upcast());
        let local = unsafe { erase_mut(ip).get_local_mut(element_id) };
        let expr = local.expr.as_mut().unwrap();
        let runner = &mut local.runner;
        let mut ctx = Self {
            ip,
            element,
            module_id,
            source,
            expr,
            runner,
        };
        ctx.run()
    }
    fn run(&mut self) -> Option<ValueStorage> {
        match &self.expr {
            Expr::Ref(_) => self.run_ref(),
            Expr::Find(_) => self.run_find(),
            Expr::Call(_) => self.run_call(),
            Expr::FunctionBody(_) => function::BodyContext::run(self),
            Expr::CompleteScope(_) => self.run_complete_scope(),
            Expr::Value(value) => {
                let ret = Some(*value);
                match value {
                    ValueStorage::Scope(scope) => self.run_scope(scope.0)?,
                    ValueStorage::Function(function) => {
                        let function = self.ip.get(function.0);
                        self.run_scope(function.scope)?;
                    }
                    _ => (),
                }
                ret
            }
        }
    }
    fn run_ref(&mut self) -> Option<ValueStorage> {
        let r#ref = self.expr.extract_as_ref();
        self.ip
            .depend_element(self.element.get_id(), r#ref.element, self.source)
    }
    fn run_find(&mut self) -> Option<ValueStorage> {
        let find = self.expr.extract_as_find();
        let meta = find.meta;
        let scope_id = self.element.source.as_ref().unwrap().scope;
        let find_element_id = if let Some(target) = find.target {
            let target = self
                .ip
                .depend_child_element(self.element.get_id(), target)?;
            match target {
                ValueStorage::Scope(value::Scope(scope_id)) => {
                    self.ip.find_element(scope_id, find.name, false)
                }
                _ => {
                    unsafe {
                        self.ip.diagnose(
                            Location::Element(self.element.get_id()),
                            Diagnostic::CanNotFindIn { value: target },
                        )
                    };
                    return None;
                }
            }
        } else {
            self.ip.find_element(scope_id, find.name, true)
        };
        if let Some(find_element_id) = find_element_id {
            if !meta {
                *self.expr = Expr::Ref(expr::Ref {
                    element: find_element_id,
                });
                self.ip
                    .depend_element(self.element.get_id(), find_element_id, self.source)
            } else {
                Some(ValueStorage::Element(value::Element(find_element_id)))
            }
        } else {
            unsafe {
                self.ip.diagnose(
                    Location::Element(self.element.get_id()),
                    Diagnostic::FailedFindElement {},
                )
            };
            return None;
        }
    }
    fn run_call(&mut self) -> Option<ValueStorage> {
        let call = self.expr.extract_as_call();
        let function = self
            .ip
            .depend_child_element(self.element.get_id(), call.function)?;
        match function {
            ValueStorage::BuiltinFunction(builtin) => {
                let param = self
                    .ip
                    .depend_child_element(self.element.get_id(), call.param)?;
                buitin_function::Context::run(self, builtin, param)
            }
            ValueStorage::Function(function) => {
                function::CallContext::run(self, function, call.param)
            }
            _ => return None,
        }
    }
    fn run_complete_scope(&mut self) -> Option<ValueStorage> {
        let scope = self.expr.extract_as_complete_scope();
        let scope = erase(self.ip.get(scope.0));
        for element in scope.elements.values().copied() {
            try {
                let value = self
                    .ip
                    .depend_element(self.element.get_id(), element, self.source)?;
                match value {
                    ValueStorage::Scope(scope) => {
                        let scope = self.ip.get(scope.0);
                        if scope.owner == self.element.owner {
                            self.ip.depend_element(
                                self.element.get_id(),
                                scope.complete,
                                self.source,
                            );
                        }
                    }
                    _ => (),
                }
            };
        }
        None
    }
    fn run_scope(&mut self, scope: Id<Scope>) -> Option<()> {
        let scope = erase(self.ip).get(scope);
        for effect in scope.effects.iter().copied() {
            self.ip
                .depend_element(self.element.get_id(), effect, None)?;
        }
        Some(())
    }
}

pub trait InterpreterLikeMut: InterpreterLikeBasicMut {
    /// # Safety
    /// - `module_id` is local.
    unsafe fn run_module(&mut self, module_id: ModuleId) {
        let module_local = unsafe { erase_mut(self).get_module_local_mut(module_id) };

        if let Some(authored) = module_local.authored {
            let root_scope =
                unsafe { erase(self.add_scope(None, Owner::Module(module_id), Some(authored))) };
            let root_scope_element = self.get_module(module_id).root_scope.unwrap();
            let root_scope_element_local = unsafe { self.get_local_mut(root_scope_element) };
            root_scope_element_local.expr = Some(Expr::Value(ValueStorage::Scope(value::Scope(
                root_scope.get_id(),
            ))));
            unsafe {
                self.run_element(root_scope_element);
                self.run_element(root_scope.complete);
            };
            module_local.unresolved_count -= 1;
            self.decrease_workload();
        }
    }
    /// # Safety
    /// - `element_id` is local.
    unsafe fn run_element(&mut self, element_id: Id<Element>) {
        let mut element_local = unsafe { self.get_local_mut(element_id) };
        if element_local.is_running {
            return;
        } else {
            element_local.is_running = true;
        }
        if element_local.dependency_count == 0 && !element_local.is_resolved() {
            element_local.is_running = true;
            let value = RunExprContext::run_expr(self, element_id);
            element_local = unsafe { erase_mut(self).get_local_mut(element_id) };
            self.set_element_value(
                element_id,
                value.unwrap_or(ValueStorage::Error(value::Error)),
            );
            element_local.is_running = false;
        }
    }

    /// # Panic
    /// - when concurrent, dependant is not in local thread.
    /// - when not concurrent, dependency id is remote.
    fn depend_element_raw(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
        source: Option<UntypedNode<'static>>,
        local: bool,
    ) -> Option<ValueStorage> {
        if self.is_local(dependency_id) {
            unsafe { self.run_element(dependency_id) };
            let dependency = erase_mut(unsafe { self.get_local_mut(dependency_id) });
            if local {
                if let Some(value) = dependency.get_resolved() {
                    return Some(value);
                } else {
                    let dependant_local = unsafe { self.get_local_mut(dependant_id) };
                    dependant_local.dependency_count += 1;
                }
            } else {
                if dependency.is_resolved() {
                    self.resolve_element(dependant_id);
                    return None;
                }
            }

            dependency.dependants.push(Dependant {
                element_id: dependant_id,
                source,
            });
        } else {
            debug_assert!(local);
            let dependency = self.get(dependency_id);
            if let Some(value) = dependency.get_resolved() {
                return Some(value);
            }
            let dependant_local = unsafe { self.get_local_mut(dependant_id) };
            dependant_local.dependency_count += 1;
            if let Some(thread) = self.get_thread_remote_of(dependency_id) {
                log::error!(
                    "thread {:?}: send depend on {}",
                    self.thread(),
                    value::Element(dependency_id).with_ctx(self)
                );
                thread.channel.push(Signal::Depend(Depend {
                    dependant: dependant_id,
                    dependency: dependency_id,
                    source,
                }));
                self.increase_workload();
            }
        }
        None
    }
    fn depend_element(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
        source: Option<UntypedNode<'static>>,
    ) -> Option<ValueStorage> {
        self.depend_element_raw(dependant_id, dependency_id, source, true)
    }
    /// # Panic
    /// - when concurrent, any element is not in local thread.
    fn depend_child_element(
        &mut self,
        dependant_id: Id<Element>,
        dependency_id: Id<Element>,
    ) -> Option<ValueStorage> {
        let dependency = self.get(dependency_id);
        let source = dependency.source.as_ref().map(|x| x.value_source.upcast());
        self.depend_element(dependant_id, dependency_id, source)
    }
    /// # Panic
    /// element is not in threads
    fn resolve_element(&mut self, id: Id<Element>) {
        if self.is_local(id) {
            let dependant = unsafe { self.get_local_mut(id) };
            dependant.dependency_count -= 1;
            unsafe { self.run_element(id) };
        } else {
            let thread = self.get_thread_remote_of(id).unwrap();
            log::error!(
                "thread {:?}: send resolve on {}",
                self.thread(),
                value::Element(id).with_ctx(self)
            );
            thread
                .channel
                .push(Signal::Resolve(Resolve { element: id }));
            self.increase_workload();
        }
    }
    /// # Panic
    /// - when concurrent, element is not in local thread.
    /// - element's value has been resolved.
    fn set_element_value(&mut self, element_id: Id<Element>, value: ValueStorage) {
        let element_local = unsafe { self.get_local_mut(element_id) };
        assert!(element_local.value.is_none());
        element_local.value = Some(value);
        if self.is_concurrent() {
            let element = self.get(element_id);
            element.value.set(value).unwrap();
        } else {
            let element = unsafe { self.get_mut(element_id) };
            element.value = OnceLock::from(value);
        }

        let element_local = unsafe { self.get_local_mut(element_id) };
        for dependant in mem::take(&mut element_local.dependants) {
            self.resolve_element(dependant.element_id);
        }
        let module = unsafe { self.get_module_local_mut(self.get(element_id).get_module(self)) };
        module.unresolved_count -= 1;
    }
}
