use crate::interpreter::{InModuleId, element::ConcurrentElementId};
use std::{fmt, ops::Deref};

use crate::{
    interpreter::{
        InterpreterLike,
        element::ElementId,
        scope::{ConcurrentScopeId, ScopeId},
    },
    utils::{concurrent_string_interner::StringId, moss},
};

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Int(i64),
    IntTy,
    String(StringId),
    StringTy,
    Scope(ConcurrentScopeId),
    ScopeTy,
    TyTy,
    Builtin(Builtin),
    Element(ElementId),
    ElementTy,
    Dyn,
    Ref {
        name: StringId,
        source: moss::Name<'static>,
    },
    DynRef {
        element: ConcurrentElementId,
    },
    FindRef {
        value: ElementId,
        key: StringId,
        key_source: moss::Name<'static>,
        source: moss::Find<'static>,
    },
    Call {
        func: ElementId,
        param: ElementId,
        source: moss::Call<'static>,
    },
    Meta {
        name: StringId,
        source: moss::Meta<'static>,
    },
    FindMeta {
        value: ElementId,
        key: StringId,
        key_source: moss::Name<'static>,
        source: moss::Find<'static>,
    },
    Err,
}

#[macro_export]
macro_rules! any_dyn { ( $( $x:expr ),* ) => {
    false $( ||
        match $x{
            $crate::interpreter::Value::Dyn|$crate::interpreter::Value::DynRef{..}=>true,
            _=>false
        }
    )* };
}

pub struct ContextedValue<'a, T: InterpreterLike + ?Sized> {
    pub value: &'a Value,
    pub ctx: &'a T,
}

impl<'a, T: InterpreterLike + ?Sized> fmt::Display for ContextedValue<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.value {
            Value::Int(x) => write!(f, "{x}"),
            Value::IntTy => write!(f, "Int"),
            Value::Scope(scope_id) => {
                let local_scope_id = scope_id.local;
                let collection = self.ctx.get_scope(local_scope_id);
                write!(f, "{{")?;
                for (key, element) in &collection.elements {
                    let element = self.ctx.get_element(element.global(local_scope_id.module));
                    write!(
                        f,
                        "{}: {}, ",
                        self.ctx.id2str(*key).deref(),
                        ContextedValue {
                            value: &element.value.value,
                            ctx: self.ctx
                        }
                    )?;
                }
                write!(f, "}}")
            }
            Value::ScopeTy => write!(f, "Scope"),
            Value::TyTy => write!(f, "Type"),
            Value::Builtin(builtin) => write!(f, "~{}", builtin),
            Value::Ref { name, .. } => {
                write!(f, "Ref({})", self.ctx.id2str(name).deref())
            }
            Value::FindRef { value, key, .. } => {
                let value = self.ctx.get_element(value);
                write!(
                    f,
                    "{}.{}",
                    ContextedValue {
                        value: &value.value.value,
                        ctx: self.ctx
                    },
                    self.ctx.id2str(key).deref()
                )
            }
            Value::Call { func, param, .. } => {
                let func_element = self.ctx.get_element(func);
                let param_element = self.ctx.get_element(param);
                write!(
                    f,
                    "({} {})",
                    ContextedValue {
                        value: &func_element.value.value,
                        ctx: self.ctx
                    },
                    ContextedValue {
                        value: &param_element.value.value,
                        ctx: self.ctx
                    }
                )
            }
            Value::Err => write!(f, "Err"),
            Value::String(string) => {
                write!(f, "{}", self.ctx.id2str(string).deref())
            }
            Value::StringTy => write!(f, "String"),
            Value::Element(element_id) => write!(f, "Element({})", {
                let element = self.ctx.get_element(element_id);
                let file = self
                    .ctx
                    .get_scope(element.scope.global(element_id.module))
                    .get_file()
                    .unwrap();
                self.ctx.get_source_str(
                    &element.authored.as_ref().unwrap().key_source.unwrap(),
                    file,
                )
            }),
            Value::ElementTy => write!(f, "Element"),
            Value::Meta { name, source } => write!(f, "@{}", &*self.ctx.id2str(name)),
            Value::FindMeta {
                value,
                key,
                key_source,
                source,
            } => {
                let value = self.ctx.get_element(value);
                write!(
                    f,
                    "{}.@{}",
                    ContextedValue {
                        value: &value.value.value,
                        ctx: self.ctx
                    },
                    self.ctx.id2str(key).deref()
                )
            }
            Value::Dyn => write!(f, "Dyn"),
            Value::DynRef { element } => write!(f, "Dyn"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Builtin {
    If,
    Add,
    Mod,
    Diagnose,
}
impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Builtin::If => write!(f, "if"),
            Builtin::Add => write!(f, "add"),
            Builtin::Mod => write!(f, "mod"),
            Builtin::Diagnose => write!(f, "diagnose"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TypedValue {
    pub value: Value,
    pub r#type: Value,
}

impl TypedValue {
    pub fn err() -> Self {
        Self {
            value: Value::Err,
            r#type: Value::Err,
        }
    }
}
