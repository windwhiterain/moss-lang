use type_sitter::{Node, UntypedNode};

use crate::{
    interpreter::{
        Id, InterpreterLikeMut, Location,
        diagnose::Diagnostic,
        element::Element,
        expr::{self, Expr},
        module::ModuleId,
        scope::Scope,
        value::{self, Value},
    },
    utils::erase_mut,
};

mod buitin_function;
mod function;

pub struct Context<'a, IP> {
    ip: &'a mut IP,
    element_id: Id<Element>,
    scope_id: Id<Scope>,
    module_id: ModuleId,
    source: Option<UntypedNode<'static>>,
    expr: &'a mut Expr,
}

impl<'a, IP: InterpreterLikeMut> Context<'a, IP> {
    pub fn run_value(ip: &'a mut IP, element_id: Id<Element>) -> Option<Value> {
        let element = ip.get(element_id);
        let scope_id = element.scope;
        let module_id = ip.get(scope_id).module;
        let source = element.source.map(|x| x.value_source.upcast());
        let expr = unsafe { erase_mut(ip).get_local_mut(element_id) }
            .expr
            .as_mut()
            .unwrap();
        let mut ctx = Self {
            ip,
            element_id,
            scope_id,
            module_id,
            source,
            expr,
        };
        match ctx.expr {
            Expr::Ref(..) => ctx.run_ref(),
            Expr::Find(..) => ctx.run_find(),
            Expr::Call(..) => ctx.run_call(),
            Expr::FunctionOptimize(..) => function::OptimizeContext::run(&mut ctx),
            Expr::Value(value) => Some(*value),
        }
    }
    fn run_ref(&mut self) -> Option<Value> {
        let r#ref = self.expr.extract_as_ref();
        self.ip
            .depend_element(self.element_id, r#ref.element_id, self.source)
    }
    fn run_find(&mut self) -> Option<Value> {
        let find = self.expr.extract_as_find();
        let find_element_id = if let Some(target) = find.target {
            let target = self.ip.depend_child_element(self.element_id, target)?;
            match target {
                Value::Scope(value::Scope(scope_id)) => {
                    self.ip.find_element(scope_id, find.name, false)
                }
                _ => {
                    if let Some(source) = self.source {
                        unsafe {
                            self.ip.diagnose(
                                Location::Element(self.element_id),
                                Diagnostic::CanNotFindIn {
                                    source,
                                    value: target,
                                },
                            )
                        };
                    }
                    return None;
                }
            }
        } else {
            self.ip.find_element(self.scope_id, find.name, true)
        };
        if let Some(find_element_id) = find_element_id {
            if !find.meta {
                *self.expr = Expr::Ref(expr::Ref {
                    element_id: find_element_id,
                });

                self.ip
                    .depend_element(self.element_id, find_element_id, self.source)
            } else {
                Some(Value::Element(value::Element(find_element_id)))
            }
        } else {
            if let Some(source) = self.source {
                unsafe {
                    self.ip.diagnose(
                        Location::Element(self.element_id),
                        Diagnostic::FailedFindElement { source },
                    )
                };
            }
            return None;
        }
    }
    fn run_call(&mut self) -> Option<Value> {
        let call = self.expr.extract_as_call();
        let function = self
            .ip
            .depend_child_element(self.element_id, call.function)?;
        match function {
            Value::BuiltinFunction(builtin) => {
                let param = self.ip.depend_child_element(self.element_id, call.param)?;
                buitin_function::Context::run(self, builtin, param)
            }
            Value::Function(function) => function::CallContext::run(self, function, call.param),
            _ => return None,
        }
    }
}
