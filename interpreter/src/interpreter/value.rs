use enum_extract_macro::EnumExtract;

use crate::{
    interpreter::{
        Id,
        element::{self, ElementKey},
        function,
        scope::{self},
    },
    utils::contexted::{Contexted, WithContext},
};
use std::{
    fmt::{self, Debug, Display, Formatter},
    ops::Deref,
};

use crate::{interpreter::InterpreterLike, utils::concurrent_string_interner::StringId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinFunction {
    Mod,
    Diagnose,
}
impl fmt::Display for BuiltinFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "~")?;
        match self {
            BuiltinFunction::Mod => write!(f, "mod"),
            BuiltinFunction::Diagnose => write!(f, "diagnose"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Int(pub i64);

impl Display for Int {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntType;
impl Display for IntType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Int")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct String(pub StringId);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, String, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", &*self.ctx.id2str(self.value.0))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringType;
impl Display for StringType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "String")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope(pub Id<scope::Scope>);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, Scope, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let scope = self.ctx.get(self.value.0);
        write!(f, "{{")?;
        for key in scope.elements.keys() {
            write!(f, "{}, ", self.ctx.id2str(*key).deref(),)?;
        }
        write!(f, "}}")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeType;
impl Display for ScopeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Scope")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element(pub Id<element::Element>);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, Element, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let element = self.ctx.get(self.value.0);
        let name = match element.key {
            ElementKey::Name(name) => &*self.ctx.id2str(name),
            ElementKey::Temp => "<Temp>",
        };
        write!(f, "@{}", name)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementType;
impl Display for ElementType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Element")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Function(pub Id<function::Function>);
impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "->{{}}")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionBody(pub Id<function::FunctionBody>);
impl Display for FunctionBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "->{{..}}")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionType;
impl Display for FunctionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Function")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeType;
impl Display for TypeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Type")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trivial;
impl Display for Trivial {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "()")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error;
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "?")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Param(pub Id<function::Param>);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, Param, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let param = self.ctx.get(self.value.0);
        let function = self.ctx.get(param.function);
        let param_name = self
            .ctx
            .id2str(*self.ctx.get(function.param).key.extract_as_name());
        write!(f, "{}", &*param_name)?;
        if let Some(r#type) = param.r#type {
            write!(f, ":")?;
            if r#type.depth > 0 {
                write!(f, "^{}", r#type.depth)?;
            }
            write!(f, " {}", r#type.value.with_ctx(self.ctx))?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumExtract)]
pub enum Value {
    Int(Int),
    IntType(IntType),
    String(String),
    StringType(StringType),
    Scope(Scope),
    ScopeType(ScopeType),
    Element(Element),
    ElementType(ElementType),
    Function(Function),
    FunctionBody(FunctionBody),
    FunctionType(FunctionType),
    TypeType(TypeType),
    BuiltinFunction(BuiltinFunction),
    Error(Error),
    Trivial(Trivial),
    Param(Param),
}

impl Value {
    pub fn merge_param(
        self,
        ctx: &(impl InterpreterLike + ?Sized),
        ret: &mut Option<Id<function::Function>>
    ) {
        if let Value::Param(param) = self {
            let function = ctx.get(param.0).function;
            if let Some(ret) = ret {
                if *ret != function {
                    if ctx.get(ctx.get(*ret).scope).depth
                        < ctx.get(ctx.get(function).scope).depth
                    {
                        *ret = function;
                    }
                }
            }else{
                *ret = Some(function);
            }
        }
    }
}

#[macro_export]
macro_rules! merge_params { ($ctx:expr, $( $x:expr ),* ) => {
        {
            let mut ret = None;
            $($x.merge_param($ctx,&mut ret);)*
            ret
        }
    }
}

impl<'a, Ctx: InterpreterLike + ?Sized> Display for Contexted<'a, Value, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self.value {
            Value::Int(value) => write!(f, "{}", value),
            Value::IntType(value) => write!(f, "{}", value),
            Value::String(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            Value::StringType(value) => write!(f, "{}", value),
            Value::Scope(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            Value::ScopeType(value) => write!(f, "{}", value),
            Value::Element(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            Value::ElementType(value) => write!(f, "{}", value),
            Value::Function(value) => write!(f, "{}", value),
            Value::FunctionBody(value) => write!(f, "{}", value),
            Value::FunctionType(value) => write!(f, "{}", value),
            Value::TypeType(value) => write!(f, "{}", value),
            Value::BuiltinFunction(value) => write!(f, "{}", value),
            Value::Error(value) => write!(f, "{}", value),
            Value::Trivial(value) => write!(f, "{}", value),
            Value::Param(value) => write!(f, "{}", value.with_ctx(self.ctx)),
        }
    }
}
