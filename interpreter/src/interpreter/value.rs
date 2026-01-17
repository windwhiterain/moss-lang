use crate::interpreter::{Id, element::Element, scope::Scope};
use std::{fmt, ops::Deref};

use crate::{
    interpreter::InterpreterLike,
    utils::{concurrent_string_interner::StringId, moss},
};

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Int(i64),
    IntTy,
    String(StringId),
    StringTy,
    Scope(Id<Scope>),
    ScopeTy,
    TyTy,
    Builtin(Builtin),
    Element(Id<Element>),
    ElementTy,
    Dyn,
    Ref {
        name: StringId,
        source: moss::Name<'static>,
    },
    DynRef {
        element: Id<Element>,
    },
    FindRef {
        value: Id<Element>,
        key: StringId,
        key_source: moss::Name<'static>,
        source: moss::Find<'static>,
    },
    Call {
        func: Id<Element>,
        param: Id<Element>,
        source: moss::Call<'static>,
    },
    Meta {
        name: StringId,
        source: moss::Meta<'static>,
    },
    FindMeta {
        value: Id<Element>,
        key: StringId,
        key_source: moss::Name<'static>,
        source: moss::Find<'static>,
    },
    Err,
}

impl Value {
    pub fn is_dyn(&self) -> bool {
        match self {
            Self::Dyn
            | Self::DynRef { .. }
            | Self::Call { .. }
            | Self::Meta { .. }
            | Self::FindMeta { .. } => true,
            _ => false,
        }
    }
    pub fn with_ctx<'a, Ctx:InterpreterLike>(&'a self,ctx:&'a Ctx)->ContextedValue<'a,Ctx>{
        ContextedValue { value: self, ctx: ctx }
    }
}

#[macro_export]
macro_rules! any_dyn { ( $( $x:expr ),* ) => {
    false $( ||
        $crate::interpreter::Value::is_dyn($x)
    )* };
}

pub struct ContextedValue<'a, T: InterpreterLike + ?Sized> {
    pub value: &'a Value,
    pub ctx: &'a T,
}

impl<'a, T: InterpreterLike> fmt::Display for ContextedValue<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.value {
            Value::Int(x) => write!(f, "{x}"),
            Value::IntTy => write!(f, "Int"),
            Value::Scope(scope_id) => {
                write!(f,"{}",scope_id.with_ctx(self.ctx))
            }
            Value::ScopeTy => write!(f, "Scope"),
            Value::TyTy => write!(f, "Type"),
            Value::Builtin(builtin) => write!(f, "~{}", builtin),
            Value::Ref { name, .. } => {
                write!(f, "Ref({})", self.ctx.id2str(name).deref())
            }
            Value::FindRef { value, key, .. } => {
                write!(
                    f,
                    "{}.{}",
                    value.with_ctx(self.ctx),
                    self.ctx.id2str(key).deref()
                )
            }
            Value::Call { func, param, .. } => {
                write!(
                    f,
                    "({} {})",
                    func.with_ctx(self.ctx),
                    param.with_ctx(self.ctx)
                )
            }
            Value::Err => write!(f, "Err"),
            Value::String(string) => {
                write!(f, "{}", self.ctx.id2str(string).deref())
            }
            Value::StringTy => write!(f, "String"),
            Value::Element(element_id) => write!(f, "Element({})", {
                let element = self.ctx.get(element_id);
                let file = self.ctx.get(element.scope).get_file().unwrap();
                self.ctx
                    .get_source_str(&element.source.as_ref().unwrap().key_source.unwrap(), file)
            }),
            Value::ElementTy => write!(f, "Element"),
            Value::Meta { name, source } => write!(f, "@{}", &*self.ctx.id2str(name)),
            Value::FindMeta {
                value,
                key,
                key_source,
                source,
            } => {
                write!(
                    f,
                    "{}.@{}",
                    value.with_ctx(self.ctx),
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
