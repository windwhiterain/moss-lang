use type_sitter::{Node, UntypedNode};

use crate::{
    interpreter::{
        Id, InterpreterLikeMut, Location, Managed,
        diagnose::Diagnostic,
        element::Element,
        expr::{self, Expr},
        function::Param,
        module::ModuleId,
        scope::Scope,
        value::{self, ValueStorage},
    },
    utils::{erase, erase_mut},
};

mod buitin_function;
mod function;

pub struct Context<'a, IP> {
    ip: &'a mut IP,
    element: &'a Element,
    module_id: ModuleId,
    source: Option<UntypedNode<'static>>,
    expr: &'a mut Expr,
}

impl<'a, IP: InterpreterLikeMut> Context<'a, IP> {
    pub fn run_expr(ip: &'a mut IP, element_id: Id<Element>) -> Option<ValueStorage> {
        let element = erase(ip).get(element_id);
        let module_id = element.module;
        let source = element.source.map(|x| x.value_source.upcast());
        let expr = unsafe { erase_mut(ip).get_local_mut(element_id) }
            .expr
            .as_mut()
            .unwrap();
        let mut ctx = Self {
            ip,
            element,
            module_id,
            source,
            expr,
        };
        ctx.run()
    }
    fn run(&mut self) -> Option<ValueStorage> {
        match &self.expr {
            Expr::Ref(_) => self.run_ref(),
            Expr::Find(_) => self.run_find(),
            Expr::Call(_) => self.run_call(),
            Expr::FunctionBody(_) => function::BodyContext::run(self),
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
            .depend_element(self.element.get_id(), r#ref.element, self.source, false)
    }
    fn run_find(&mut self) -> Option<ValueStorage> {
        let find = self.expr.extract_as_find();
        let meta = find.meta;
        let scope_id = self.element.source.as_ref().unwrap().scope;
        let find_element_id = if let Some(target) = find.target {
            let target = self
                .ip
                .depend_child_element(self.element.get_id(), target, false)?;
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
                    .depend_element(self.element.get_id(), find_element_id, self.source, false)
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
            .depend_child_element(self.element.get_id(), call.function, true)?;
        match function {
            ValueStorage::BuiltinFunction(builtin) => {
                let param =
                    self.ip
                        .depend_child_element(self.element.get_id(), call.param, false)?;
                buitin_function::Context::run(self, builtin, param)
            }
            ValueStorage::Function(function) => {
                function::CallContext::run(self, function, call.param)
            }
            _ => return None,
        }
    }
    fn run_scope(&mut self, scope: Id<Scope>) -> Option<()> {
        let scope = erase(self.ip).get(scope);
        for effect in scope.effects.iter().copied() {
            self.ip
                .depend_element(self.element.get_id(), effect, None, false)?;
        }
        for element in scope.elements.values().copied() {
            self.ip
                .depend_element(self.element.get_id(), element, None, true);
        }
        Some(())
    }
}
