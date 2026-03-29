use std::path::Path;

use type_sitter::UntypedNode;

use crate::{
    interpreter::{
        Id, InterpreterLikeMut, Location, Managed as _, SRC_FILE_EXTENSION, SRC_PATH,
        diagnose::Diagnostic,
        element::Element,
        function::Param,
        module::ModuleId,
        value::{self, BuiltinFunction, ValueStorage},
    },
    merge_params,
    utils::erase,
};

pub struct Context<'a, IP> {
    ip: &'a mut IP,
    element_id: Id<Element>,
    module_id: ModuleId,
    source: Option<UntypedNode<'static>>,
    param: ValueStorage,
}

impl<'a, 'b: 'a, IP: InterpreterLikeMut> Context<'a, IP> {
    pub fn run(
        ctx: &'a mut super::Context<'b, IP>,
        builtin_function: BuiltinFunction,
        param: ValueStorage,
    ) -> Option<ValueStorage> {
        let mut ctx = Self {
            ip: ctx.ip,
            element_id: ctx.element.get_id(),
            module_id: ctx.module_id,
            source: ctx.source,
            param,
        };
        match builtin_function {
            BuiltinFunction::Mod => ctx.run_mod(),
            BuiltinFunction::Diagnose => ctx.run_diagnose(),
            BuiltinFunction::Equal => ctx.run_equal(),
            BuiltinFunction::Switch => ctx.run_switch(),
        }
    }
    fn run_mod(&mut self) -> Option<ValueStorage> {
        if let Some(function) = merge_params!(self.ip, self.param) {
            return Some(ValueStorage::Param(value::ParamStorage(
                unsafe {
                    self.ip.add(
                        Param {
                            function,
                            element: self.element_id,
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
            .depend_element(self.element_id, module.root_scope.unwrap(), self.source)?
            .as_scope()
            .ok()?
            .0;

        Some(ValueStorage::Scope(value::Scope(root_scope)))
    }
    fn run_diagnose(&mut self) -> Option<ValueStorage> {
        let scope = self.param.as_scope().ok()?.0;
        let source_key = self.ip.str2id("source");
        let text_key = self.ip.str2id("text");
        let text = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(scope, text_key, false)?,
            self.source,
        )?;
        let source_element = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(scope, source_key, false)?,
            self.source,
        )?;
        if let Some(function) = merge_params!(self.ip, text, source_element) {
            return Some(ValueStorage::Param(value::ParamStorage(unsafe {
                self.ip
                    .add(
                        Param {
                            function,
                            element: self.element_id,
                            r#type: Some(ValueStorage::Diagnostic(value::Diagnostic)),
                        },
                        self.module_id,
                    )
                    .get_id()
            })));
        }
        let source_element = source_element.as_element().ok()?.0;
        let text = text.as_string().ok()?.0;
        if self.ip.is_local(source_element) {
            unsafe {
                self.ip.diagnose(
                    Location::Element(source_element),
                    Diagnostic::Custom { text },
                )
            };
        }
        Some(ValueStorage::Trivial(value::Trivial))
    }
    fn run_equal(&mut self) -> Option<ValueStorage> {
        let set = self.param.as_set().ok()?;
        let set = erase(self.ip.get(set.0));
        let mut elements = set.elements.iter().copied();
        let mut equal = true;
        if let Some(element) = elements.next() {
            let value = self
                .ip
                .depend_element(self.element_id, element, self.source)?;
            for element in elements {
                let other_value = self
                    .ip
                    .depend_element(self.element_id, element, self.source)?;
                if other_value != value {
                    equal = false;
                }
            }
        }
        Some(ValueStorage::Int(value::Int(if equal { 1 } else { 0 })))
    }
    fn run_switch(&mut self) -> Option<ValueStorage>{
        let scope = self.param.as_scope().ok()?.0;
        let index_key = self.ip.str2id("index");
        let set_key = self.ip.str2id("set");
        let index = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(scope, index_key, false)?,
            self.source,
        )?;
        let set = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(scope, set_key, false)?,
            self.source,
        )?;
        if let Some(function) = merge_params!(self.ip, index, set) {
            return Some(ValueStorage::Param(value::ParamStorage(unsafe {
                self.ip
                    .add(
                        Param {
                            function,
                            element: self.element_id,
                            r#type: None,
                        },
                        self.module_id,
                    )
                    .get_id()
            })));
        }
        let index= index.as_int().ok()?.0;
        let set = set.as_set().ok()?.0;
        let set = self.ip.get(set);
        if let Some(value) = set.elements.get(index).copied(){
            Some(self.ip.depend_element(self.element_id, value, self.source)?)
        }else{
            Some(ValueStorage::Error(value::Error))
        }
    }
}
