use std::path::Path;

use moss_parser::UntypedNode;

use crate::{
    interpreter::{
        Id, Location, Managed as _, SRC_FILE_EXTENSION, SRC_PATH,
        element::Element,
        error::{self, Error},
        expr::{self, Expr},
        function::Param,
        module::ModuleId,
        run,
        value::{self, BuiltinFunction, ValueStorage},
    },
    merge_params, try_tuple,
    utils::erase,
};

pub struct Context<'a, IP> {
    ip: &'a mut IP,
    element_id: Id<Element>,
    module_id: ModuleId,
    source: Option<UntypedNode<'static>>,
    param: ValueStorage,
    expr: &'a mut Expr,
}

impl<'a, 'b: 'a, IP: run::InterpreterLikeMut> Context<'a, IP> {
    pub fn run(
        ctx: &'a mut super::RunExprContext<'b, IP>,
        builtin_function: BuiltinFunction,
        param: ValueStorage,
    ) -> Option<ValueStorage> {
        let mut ctx = Self {
            ip: ctx.ip,
            element_id: ctx.element.get_id(),
            module_id: ctx.module_id,
            source: ctx.source,
            param,
            expr: ctx.expr,
        };
        match builtin_function {
            BuiltinFunction::Mod => ctx.run_mod(),
            BuiltinFunction::Error => ctx.run_error(),
            BuiltinFunction::Equal => ctx.run_equal(),
            BuiltinFunction::Switch => ctx.run_switch(),
            BuiltinFunction::TypeOf => ctx.run_type_of(),
            BuiltinFunction::WithType => ctx.run_with_type(),
            BuiltinFunction::Find => ctx.run_find(),
            BuiltinFunction::ValueOf => ctx.run_value_of(),
        }
    }
    fn run_mod(&mut self) -> Option<ValueStorage> {
        if let Some(function) = merge_params!(self.ip, self.param) {
            return Some(ValueStorage::Param(value::Param(
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
        let module_id = self.ip.get_file(file).module?;
        let module = self.ip.get_module(module_id);
        let root_scope = self
            .ip
            .depend_element(self.element_id, module.root_scope.unwrap(), self.source)?
            .as_scope()
            .ok()?
            .0;

        Some(ValueStorage::Scope(value::Scope(root_scope)))
    }
    fn run_error(&mut self) -> Option<ValueStorage> {
        let scope = self.param.as_scope().ok()?.0;
        let location_key = self.ip.str2id("location");
        let message_key = self.ip.str2id("message");
        let (source_element, text) = try_tuple!(
            self.ip.depend_element(
                self.element_id,
                self.ip.find_element(scope, location_key, false)?,
                self.source,
            ),
            self.ip.depend_element(
                self.element_id,
                self.ip.find_element(scope, message_key, false)?,
                self.source,
            ),
        )?;
        if let Some(function) = merge_params!(self.ip, source_element, text) {
            return Some(ValueStorage::Param(value::Param(unsafe {
                self.ip
                    .add(
                        Param {
                            function,
                            element: self.element_id,
                            r#type: Some(ValueStorage::ErrorType(value::ErrorType)),
                        },
                        self.module_id,
                    )
                    .get_id()
            })));
        }
        let source = source_element.as_element().ok()?.0;
        let text = text.as_string().ok()?.0;
        let error = unsafe {
            self.ip.add(
                Error {
                    kind: error::Kind::Custom { text },
                    location: Location::Element(source),
                },
                self.module_id,
            )
        }
        .get_id();
        Some(ValueStorage::Error(value::Error(error)))
    }
    fn run_equal(&mut self) -> Option<ValueStorage> {
        let set = self.param.as_set().ok()?;
        let set = erase(self.ip.get(set.0));
        let mut equal = true;
        let mut param = None;
        let mut value = None;
        for element in set.elements.iter().copied() {
            let other_value = self
                .ip
                .depend_element(self.element_id, element, self.source)?;
            if let ValueStorage::Param(_) = other_value {
                other_value.merge_param(self.ip, &mut param);
            } else {
                if let Some(value) = value {
                    if value != other_value {
                        equal = false;
                        break;
                    }
                } else {
                    value = Some(other_value);
                }
            }
        }
        Some(if !equal {
            ValueStorage::Int(value::Int(0))
        } else {
            if let Some(function) = param {
                ValueStorage::Param(value::Param(unsafe {
                    self.ip
                        .add(
                            Param {
                                function,
                                element: self.element_id,
                                r#type: Some(ValueStorage::IntType(value::IntType)),
                            },
                            self.module_id,
                        )
                        .get_id()
                }))
            } else {
                ValueStorage::Int(value::Int(1))
            }
        })
    }
    fn run_switch(&mut self) -> Option<ValueStorage> {
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
        if let Some(function) = merge_params!(self.ip, set) {
            return Some(ValueStorage::Param(value::Param(unsafe {
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
        let set = set.as_set().ok()?.0;
        let set = self.ip.get(set);
        if let Some(function) = merge_params!(self.ip, index) {
            for element in erase(&set.elements).iter().copied() {
                self.ip
                    .depend_element(self.element_id, element, self.source)?;
            }
            return Some(ValueStorage::Param(value::Param(unsafe {
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
        let index = index.as_int().ok()?.0;
        if let Some(element) = set.elements.get(index).copied() {
            *self.expr = Expr::Ref(expr::Ref { element });
            Some(
                self.ip
                    .depend_element(self.element_id, element, self.source)?,
            )
        } else {
            Some(ValueStorage::Trivial(value::Trivial))
        }
    }
    fn run_type_of(&mut self) -> Option<ValueStorage> {
        if let ValueStorage::Param(param) = self.param {
            let param = self.ip.get(param.0);
            Some(ValueStorage::Param(value::Param(unsafe {
                self.ip
                    .add(
                        Param {
                            function: param.function,
                            element: self.element_id,
                            r#type: None,
                        },
                        self.module_id,
                    )
                    .get_id()
            })))
        } else {
            Some(self.param.get_type(self.ip).unwrap())
        }
    }
    fn run_with_type(&mut self) -> Option<ValueStorage> {
        let scope = self.param.as_scope().ok()?.0;
        let value_key = self.ip.str2id("value");
        let value = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(scope, value_key, false)?,
            self.source,
        )?;
        let ValueStorage::Param(value_param) = value else {
            return Some(value);
        };
        let value_param = erase(self.ip.get(value_param.0));
        let type_key = self.ip.str2id("type");
        let r#type = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(scope, type_key, false)?,
            self.source,
        )?;
        Some(ValueStorage::Param(value::Param(unsafe {
            self.ip
                .add(
                    Param {
                        function: value_param.function,
                        element: self.element_id,
                        r#type: Some(r#type),
                    },
                    self.module_id,
                )
                .get_id()
        })))
    }
    fn run_find(&mut self) -> Option<ValueStorage> {
        let params = self.param.as_scope().ok()?.0;
        let scope_key = self.ip.str2id("scope");
        let scope = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(params, scope_key, false)?,
            self.source,
        )?;
        let key_key = self.ip.str2id("key");
        let key = self.ip.depend_element(
            self.element_id,
            self.ip.find_element(params, key_key, false)?,
            self.source,
        )?;
        if let Some(function) = merge_params!(self.ip, scope, key) {
            return Some(ValueStorage::Param(value::Param(unsafe {
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
        let scope = scope.as_scope().ok()?.0;
        let key = key.as_string().ok()?.0;
        if let Some(element) = self.ip.find_element(scope, key, false) {
            Some(ValueStorage::Element(value::Element(element)))
        } else {
            None
        }
    }
    fn run_value_of(&mut self) -> Option<ValueStorage> {
        if let Some(function) = merge_params!(self.ip, self.param) {
            return Some(ValueStorage::Param(value::Param(unsafe {
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
        let element = self.param.as_element().ok()?.0;
        self.ip
            .depend_element(self.element_id, element, self.source)
    }
}
