use enum_extract_macro::EnumExtract;

use crate::{
    interpreter::{
        Id,
        element::{self, ElementKey},
        function::{self},
        scope::{self}, set,
    },
    utils::contexted::{Contexted, WithContext},
};
use std::{
    fmt::{self, Debug, Display, Formatter},
    ops::Deref,
};

use crate::{interpreter::InterpreterLike, utils::concurrent_string_interner::StringId};

pub trait ValueLike {
    fn is_effective_type()->bool{false}
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinFunction {
    Mod,
    Diagnose,
    Equal,
    Switch,
}
impl fmt::Display for BuiltinFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*")?;
        match self {
            BuiltinFunction::Mod => write!(f, "mod"),
            BuiltinFunction::Diagnose => write!(f, "diagnose"),
            BuiltinFunction::Equal => write!(f, "equal"),
            BuiltinFunction::Switch => write!(f,"switch"),
        }
    }
}

impl ValueLike for BuiltinFunction{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::FunctionType(FunctionType)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Int(pub usize);

impl Display for Int {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl ValueLike for Int{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::IntType(IntType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntType;
impl Display for IntType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Int")
    }
}
impl ValueLike for IntType{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct String(pub StringId);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, String, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", &*self.ctx.id2str(self.value.0))
    }
}
impl ValueLike for String{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::StringType(StringType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringType;
impl Display for StringType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "String")
    }
}
impl ValueLike for StringType{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Set(pub Id<set::Set>);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, Set, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let set = self.ctx.get(self.value.0);
        write!(f, "{{}}^{}",set.elements.len())
    }
}
impl ValueLike for Set{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::SetType(SetType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetType;
impl Display for SetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Set")
    }
}
impl ValueLike for SetType{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope(pub Id<scope::Scope>);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, Scope, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let scope = self.ctx.get(self.value.0);
        write!(f, "{{")?;
        let mut elements = scope.elements.keys();
        if let Some(first) = elements.next(){
            write!(f, "{};", self.ctx.id2str(*first).deref(),)?;
            for key in elements {
            write!(f, " {};", self.ctx.id2str(*key).deref(),)?;
        }
        }else{
            write!(f,";")?
        }
        write!(f, "}}")
    }
}
impl ValueLike for Scope{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::ScopeType(ScopeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeType;
impl Display for ScopeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Scope")
    }
}
impl ValueLike for ScopeType{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element(pub Id<element::Element>);
impl<'a, Ctx: ?Sized + InterpreterLike> Display for Contexted<'a, Element, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let element = self.ctx.get(self.value.0);
        let name = match element.key {
            ElementKey::Name(name) => &*self.ctx.id2str(name),
            ElementKey::Effect => "<Effect>",
            ElementKey::Temp => "<Temp>",
        };
        write!(f, "@{}", name)
    }
}
impl ValueLike for Element{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::ElementType(ElementType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementType;
impl Display for ElementType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Element")
    }
}
impl ValueLike for ElementType{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Function(pub Id<function::Function>);
impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "->{{}}")
    }
}
impl ValueLike for Function{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::FunctionType(FunctionType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionBody(pub Id<function::FunctionBody>);
impl Display for FunctionBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "->{{..}}")
    }
}
impl ValueLike for FunctionBody{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::Trivial(Trivial)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionType;
impl Display for FunctionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Function")
    }
}
impl ValueLike for FunctionType{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeType;
impl Display for TypeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Type")
    }
}
impl ValueLike for TypeType{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trivial;
impl Display for Trivial {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "()")
    }
}
impl ValueLike for Trivial{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::Trivial(Trivial)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error;
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "?")
    }
}
impl ValueLike for Error{
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::Error(Error)
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
        write!(f, "{}:", &*param_name)?;
        if let Some(r#type) = param.r#type {
            write!(f, " {}", r#type.with_ctx(self.ctx))?;
        }else{
            write!(f, " ?")?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostic;
impl Display for Diagnostic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Diagnostic")
    }
}
impl ValueLike for Diagnostic{
    fn is_effective_type()->bool {
        true
    }
    
    fn get_type(self,ctx:&impl InterpreterLike)->ValueStorage {
        ValueStorage::TypeType(TypeType)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumExtract)]
pub enum ValueStorage {
    Int(Int),
    IntType(IntType),
    String(String),
    StringType(StringType),
    Set(Set),
    SetType(SetType),
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
    Diagnostic(Diagnostic),
}

impl ValueStorage {
    pub fn merge_param(
        self,
        ctx: &(impl InterpreterLike + ?Sized),
        ret: &mut Option<Id<function::Function>>,
    ) {
        if let ValueStorage::Param(param) = self {
            let function = ctx.get(param.0).function;
            if let Some(ret) = ret {
                if *ret != function {
                    if ctx.get(ctx.get(*ret).scope).depth < ctx.get(ctx.get(function).scope).depth {
                        *ret = function;
                    }
                }
            } else {
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

impl<'a, Ctx: InterpreterLike + ?Sized> Display for Contexted<'a, ValueStorage, Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self.value {
            ValueStorage::Int(value) => write!(f, "{}", value),
            ValueStorage::IntType(value) => write!(f, "{}", value),
            ValueStorage::String(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            ValueStorage::StringType(value) => write!(f, "{}", value),
            ValueStorage::Set(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            ValueStorage::SetType(value) => write!(f, "{}", value),
            ValueStorage::Scope(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            ValueStorage::ScopeType(value) => write!(f, "{}", value),
            ValueStorage::Element(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            ValueStorage::ElementType(value) => write!(f, "{}", value),
            ValueStorage::Function(value) => write!(f, "{}", value),
            ValueStorage::FunctionBody(value) => write!(f, "{}", value),
            ValueStorage::FunctionType(value) => write!(f, "{}", value),
            ValueStorage::TypeType(value) => write!(f, "{}", value),
            ValueStorage::BuiltinFunction(value) => write!(f, "{}", value),
            ValueStorage::Error(value) => write!(f, "{}", value),
            ValueStorage::Trivial(value) => write!(f, "{}", value),
            ValueStorage::Param(value) => write!(f, "{}", value.with_ctx(self.ctx)),
            ValueStorage::Diagnostic(value)=>write!(f,"{}",value),
        }
    }
}
