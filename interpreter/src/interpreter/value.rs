use enum_extract_macro::EnumExtract;

use crate::interpreter::{
    Id,
    element::{Element, ElementKey},
    scope::Scope,
};
use std::{
    fmt::{self, Debug, Display, Formatter},
    ops::Deref,
};

use crate::{
    interpreter::InterpreterLike,
    utils::{concurrent_string_interner::StringId, moss},
};

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

#[derive(Clone, Copy, Debug, EnumExtract)]
pub enum StaticValue {
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
    Err,
    Trivial,
}

#[derive(Clone, Copy, Debug)]
pub struct Type {
    pub value: StaticValue,
    pub depth: usize,
}

#[derive(Clone, Copy, Debug, EnumExtract)]
pub enum Value {
    Static(StaticValue),
    In {
        scope: Id<Scope>,
        r#type: Option<Type>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum Expr {
    Ref {
        name: StringId,
        source: moss::Name<'static>,
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
    Value(StaticValue),
}

impl StaticValue {
    pub fn with_ctx<'a, Ctx: InterpreterLike>(
        &'a self,
        ctx: &'a Ctx,
    ) -> ContextedStaticValue<'a, Ctx> {
        ContextedStaticValue {
            value: self,
            ctx: ctx,
        }
    }
}
impl Value {
    pub fn merge_in(
        self,
        ctx: &(impl InterpreterLike + ?Sized),
        other: Option<Value>,
    ) -> Option<Id<Scope>> {
        let other_scope = other.map(|x| x.merge_in(ctx, None)).flatten();
        if let Value::In { scope, .. } = self {
            if let Some(other_scope) = other_scope {
                if other_scope != scope {
                    if ctx.get(other_scope).depth > ctx.get(scope).depth {
                        return Some(other_scope);
                    }
                }
            }
            Some(scope)
        } else {
            other_scope
        }
    }
}

impl Value {
    pub fn with_ctx<'a, Ctx>(&'a self, ctx: &'a Ctx) -> ContextedValue<'a, Ctx> {
        ContextedValue { value: self, ctx }
    }
}

#[macro_export]
macro_rules! merge_in { ($ctx:expr, $( $x:expr ),* ) => {
    $crate::interpreter::value::Value::Static($crate::interpreter::value::StaticValue::Trivial)$(.merge_in(
        $ctx,Some($x))
    )* };
}

pub struct ContextedStaticValue<'a, T: InterpreterLike + ?Sized> {
    pub value: &'a StaticValue,
    pub ctx: &'a T,
}

impl<'a, T: InterpreterLike> Display for ContextedStaticValue<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self.value {
            StaticValue::Int(x) => write!(f, "{x}"),
            StaticValue::IntTy => write!(f, "Int"),
            StaticValue::Scope(scope_id) => {
                write!(f, "{}", scope_id.with_ctx(self.ctx))
            }
            StaticValue::ScopeTy => write!(f, "Scope"),
            StaticValue::TyTy => write!(f, "Type"),
            StaticValue::Builtin(builtin) => write!(f, "~{}", builtin),
            StaticValue::Err => write!(f, "Err"),
            StaticValue::String(string) => {
                write!(f, "{}", self.ctx.id2str(string).deref())
            }
            StaticValue::StringTy => write!(f, "String"),
            StaticValue::Element(element_id) => write!(
                f,
                "{}",
                {
                    let element = self.ctx.get(element_id);
                    let ElementKey::Name(name) = element.key else {
                        unreachable!()
                    };
                    self.ctx.id2str(name)
                }
                .deref()
            ),
            StaticValue::ElementTy => write!(f, "Element"),
            StaticValue::Trivial => write!(f, "Trivial"),
        }
    }
}

pub struct ContextedValue<'a, Ctx> {
    pub value: &'a Value,
    pub ctx: &'a Ctx,
}

impl<'a, Ctx: InterpreterLike> Display for ContextedValue<'a, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self.value {
            Value::Static(static_value) => static_value.with_ctx(self.ctx).fmt(f),
            Value::In { scope, r#type } => {
                write!(f, "In{{depth: {}}}", self.ctx.get(scope).depth)?;
                if let Some(r#type) = r#type {
                    write!(f, ":")?;
                    if r#type.depth > 0 {
                        write!(f, "^{}", r#type.depth)?;
                    }
                    write!(f, " ")?;
                    write!(f, ": {}", r#type.value.with_ctx(self.ctx))?;
                }
                Ok(())
            }
        }
    }
}
