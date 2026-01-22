use std::{ops::Deref as _, path::Path};

use type_sitter::{Node as _, UntypedNode};

use crate::{
    interpreter::{
        Id, InterpreterLikeMut, Location, Managed as _, SRC_FILE_EXTENSION, SRC_PATH,
        diagnose::Diagnostic,
        element::Element,
        expr::Expr,
        function::Param,
        module::ModuleId,
        run::buitin_function,
        scope::Scope,
        value::{self, BuiltinFunction, Value},
    },
    merge_params,
    utils::erase,
};

pub struct Context<'a, IP> {
    ip: &'a mut IP,
    element_id: Id<Element>,
    scope_id: Id<Scope>,
    module_id: ModuleId,
    source: Option<UntypedNode<'static>>,
    expr: &'a mut Expr,
    builtin_function: BuiltinFunction,
    param: Value,
}

impl<'a, 'b: 'a, IP: InterpreterLikeMut> Context<'a, IP> {
    pub fn run(
        ctx: &'a mut super::Context<'b, IP>,
        builtin_function: BuiltinFunction,
        param: Value,
    ) -> Option<Value> {
        let mut ctx = Self {
            ip: ctx.ip,
            element_id: ctx.element_id,
            scope_id: ctx.scope_id,
            module_id: ctx.module_id,
            source: ctx.source,
            expr: ctx.expr,
            builtin_function,
            param,
        };
        match builtin_function {
            BuiltinFunction::Mod => ctx.run_mod(),
            BuiltinFunction::Diagnose => ctx.run_diagnose(),
        }
    }
    fn run_mod(&mut self) -> Option<Value> {
        if let Some(function) = merge_params!(self.ip, self.param) {
            return Some(Value::Param(value::Param(
                unsafe {
                    self.ip.add(
                        Param {
                            function,
                            r#type: None,
                        },
                        self.module_id,
                    )
                }
                .get_id(),
            )));
        }
        let path = self.param.as_string().ok()?.0;
        let path = Path::new(SRC_PATH)
            .join(&*self.ip.id2str(path))
            .with_extension(SRC_FILE_EXTENSION);
        let file = self.ip.find_file(path)?;
        let module_id = self.ip.get_file(file).is_module?;
        let module = self.ip.get_module(module_id);
        let root_scope = self
            .ip
            .depend_element_value(self.element_id, module.root_scope.unwrap(), self.source)?
            .as_scope()
            .ok()?
            .0;

        Some(Value::Scope(value::Scope(root_scope)))
    }
    fn run_diagnose(&mut self) -> Option<Value> {
        let scope = self.param.as_scope().ok()?.0;
        let on_key = self.ip.str2id("on");
        let source_key = self.ip.str2id("source");
        let text_key = self.ip.str2id("text");

        let on = self
            .ip
            .depend_element_value(
                self.element_id,
                self.ip.find_element(scope, on_key, false)?,
                self.source,
            )?
            .as_int()
            .ok()?
            .0;
        let text = self
            .ip
            .depend_element_value(
                self.element_id,
                self.ip.find_element(scope, text_key, false)?,
                self.source,
            )?
            .as_string()
            .ok()?
            .0;
        let source_element = self
            .ip
            .depend_element_value(
                self.element_id,
                self.ip.find_element(scope, source_key, false)?,
                self.source,
            )?
            .as_element()
            .ok()?
            .0;
        if on != 0 && self.ip.is_local(source_element) {
            unsafe {
                self.ip.diagnose(
                    Location::Element(source_element),
                    Diagnostic::Custom {
                        source: self
                            .ip
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
        Some(Value::Trivial(value::Trivial))
    }
}
