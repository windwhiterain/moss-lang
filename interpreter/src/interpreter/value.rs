use crate::interpreter::InModuleId;
use std::{fmt, ops::Deref};

use crate::{
    interpreter::{
        InterpreterLike,
        element::{Element, ElementId, InModuleElementId},
        scope::{LocalScopeId, Scope, ScopeId},
    },
    utils::{concurrent_string_interner::StringId, moss},
};

#[derive(Clone, Copy,Debug)]
pub enum Value {
    Int(i64),
    IntTy,
    String(StringId),
    StringTy,
    Scope(ScopeId),
    ScopeTy,
    TyTy,
    Builtin(Builtin),
    Name {
        name: StringId,
        scope: LocalScopeId,
        node: moss::Name<'static>,
    },
    Find {
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
    Err,
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
                            value: &element.resolved_value.value,
                            ctx: self.ctx
                        }
                    )?;
                }
                write!(f, "}}")
            }
            Value::ScopeTy => write!(f, "Scope"),
            Value::TyTy => write!(f, "Type"),
            Value::Builtin(builtin) => write!(f, "@{}", builtin),
            Value::Name { name, scope, .. } => {
                write!(f, "{}", self.ctx.id2str(name).deref())
            }
            Value::Find {
                value: element,
                key,
                ..
            } => {
                let element = self.ctx.get_element(element);
                write!(
                    f,
                    "{}.{}",
                    ContextedValue {
                        value: &element.resolved_value.value,
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
                        value: &func_element.resolved_value.value,
                        ctx: self.ctx
                    },
                    ContextedValue {
                        value: &param_element.resolved_value.value,
                        ctx: self.ctx
                    }
                )
            }
            Value::Err => write!(f, "Err"),
            Value::String(string) => {
                write!(f, "{}", self.ctx.id2str(string).deref())
            }
            Value::StringTy => write!(f, "String"),
        }
    }
}

#[derive(Clone, Copy,Debug)]
pub enum Builtin {
    If,
    Add,
    Mod,
}
impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Builtin::If => write!(f, "if"),
            Builtin::Add => write!(f, "add"),
            Builtin::Mod => write!(f, "mod"),
        }
    }
}

#[derive(Clone, Copy,Debug)]
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
