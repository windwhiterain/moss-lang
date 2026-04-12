use crate::interpreter::Owner;
use crate::interpreter::error::Error;
use crate::interpreter::set::Set;
use crate::utils::typed_key::Vec as KeyVec;
use crate::{
    interpreter::{
        Id, Managed,
        element::{Element, ElementKey},
        expr::Expr,
        module::ModuleId,
        scope::Scope,
        value::{self, ValueStorage},
    },
    utils::unsafe_cell::UnsafeCell,
};

#[derive(Debug, Clone, Copy)]
pub enum FunctionElementAuthored {
    Expr(Expr),
    Value(ValueStorage),
    MappedValue(ValueStorage),
    Capture(Id<Element>),
}

#[derive(Debug, Default)]
pub struct FunctionSet {
    pub elements: Vec<Id<Element>>,
}
#[derive(Debug)]
pub struct FunctionElement {
    pub authored: FunctionElementAuthored,
    pub key: ElementKey,
}

impl FunctionElement {
    pub const DUMMY: Self = Self {
        authored: FunctionElementAuthored::Value(ValueStorage::Trivial(value::Trivial)),
        key: ElementKey::Temp,
    };
}

#[derive(Debug)]
pub struct FunctionScope {
    pub elements: Vec<Id<Element>>,
    pub effects: Vec<Id<Element>>,
}

impl FunctionScope {
    pub const DUMMY: Self = Self {
        elements: Default::default(),
        effects: Default::default(),
    };
}

#[derive(Debug)]
pub struct FunctionFunction {
    pub param: Id<Param>,
    pub scope: Id<Scope>,
}

impl FunctionFunction {
    pub const DUMMY: Self = Self {
        param: Id::DUMMY,
        scope: Id::DUMMY,
    };
}

#[derive(Debug)]
pub struct FunctionBody {
    pub sets: KeyVec<Id<Set>, FunctionSet>,
    pub scopes: KeyVec<Id<Scope>, FunctionScope>,
    pub elements: KeyVec<Id<Element>, FunctionElement>,
    pub functions: KeyVec<Id<Function>, FunctionFunction>,
    pub errors: KeyVec<Id<Error>, Error>,
    pub root_scope: Option<Id<Scope>>,
}

impl FunctionBody {
    pub fn new() -> Self {
        Self {
            sets: Default::default(),
            scopes: Default::default(),
            elements: Default::default(),
            functions: Default::default(),
            errors: Default::default(),
            root_scope: Default::default(),
        }
    }
}

impl Managed for FunctionBody {
    type Local = ();

    const NAME: &str = "FunctionBody";

    fn get_local(&self) -> &UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_module<IP: super::InterpreterLike>(&self, _ip: &IP) -> ModuleId
    where
        Self: Sized,
    {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Function {
    pub scope: Id<Scope>,
    pub param: Id<Param>,
    pub body: Id<Element>,
    pub owner: Owner,
}

impl Function {
    pub fn new(owner: Owner, scope: Id<Scope>, param: Id<Param>, body: Id<Element>) -> Self {
        Self {
            owner,
            scope,
            param,
            body,
        }
    }
}

impl Managed for Function {
    type Local = ();

    const NAME: &str = "Function";

    fn get_local(&self) -> &UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_module<IP: super::InterpreterLike>(&self, ip: &IP) -> ModuleId
    where
        Self: Sized,
    {
        self.owner.module(ip)
    }
}

#[derive(Debug)]
pub struct Param {
    pub function: Id<Function>,
    pub element: Id<Element>,
    pub r#type: Option<ValueStorage>,
}

impl Managed for Param {
    type Local = ();

    const NAME: &str = "Param";

    fn get_local(&self) -> &UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_module<IP: super::InterpreterLike>(&self, ip: &IP) -> ModuleId
    where
        Self: Sized,
    {
        ip.get(self.function).get_module(ip)
    }
}
