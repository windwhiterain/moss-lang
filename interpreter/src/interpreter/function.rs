use crate::utils::typed_key::Vec as KeyVec;
use crate::{
    interpreter::{
        Id, Managed,
        element::{Element, ElementKey},
        expr::Expr,
        module::ModuleId,
        scope::Scope,
        value::{self, Value},
    },
    utils::unsafe_cell::UnsafeCell,
};

#[derive(Debug)]
pub enum FunctionElementAuthored {
    Expr(Expr),
    Value(Value),
    Capture(usize),
}

#[derive(Debug)]
pub struct FunctionElement {
    pub authored: FunctionElementAuthored,
    pub key: ElementKey,
}

impl FunctionElement {
    pub const DUMMY: Self = Self {
        authored: FunctionElementAuthored::Value(Value::Trivial(value::Trivial)),
        key: ElementKey::Temp,
    };
}

#[derive(Debug)]
pub struct FunctionScope {
    pub elements: Vec<Id<Element>>,
}

impl FunctionScope {
    pub const DUMMY: Self = Self {
        elements: Default::default(),
    };
}

#[derive(Debug)]
pub struct FunctionFunction {
    pub body: Id<Element>,
    pub captures: Vec<Id<Element>>,
}

impl FunctionFunction {
    pub fn new(body: Id<Element>) -> Self {
        Self {
            body,
            captures: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct FunctionBody {
    pub scopes: KeyVec<Id<Scope>, FunctionScope>,
    pub elements: KeyVec<Id<Element>, FunctionElement>,
    pub functions: KeyVec<Id<Function>, FunctionFunction>,
    pub root_scope: Option<Id<Scope>>,
}

impl FunctionBody {
    pub const PARAM_ELEMENT_ID: Id<Element> = Id::from_idx(usize::MAX);
    pub fn new() -> Self {
        Self {
            scopes: Default::default(),
            elements: Default::default(),
            functions: Default::default(),
            root_scope: Default::default(),
        }
    }
}

impl Managed for FunctionBody {
    type Local = ();

    type Onwer = Self;

    const NAME: &str = "FunctionBody";

    fn get_local(&self) -> &UnsafeCell<Self::Local> {
        todo!()
    }

    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local> {
        todo!()
    }

    fn get_owner(&self) -> super::Owner<Self::Onwer>
    where
        Self: Sized,
    {
        todo!()
    }
}

#[derive(Debug)]
pub struct Function {
    pub scope: Id<Scope>,
    pub param: Id<Element>,
    pub module: ModuleId,
    pub body: Id<Element>,
    pub captures: UnsafeCell<Vec<Id<Element>>>,
}

impl Function {
    pub fn new(scope: Id<Scope>, param: Id<Element>, module: ModuleId, body: Id<Element>) -> Self {
        Self {
            scope,
            param,
            module,
            body,
            captures: UnsafeCell::new(Default::default()),
        }
    }
}

impl Managed for Function {
    type Local = ();

    type Onwer = Function;

    const NAME: &str = "Function";

    fn get_local(&self) -> &UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_owner(&self) -> super::Owner<Self::Onwer>
    where
        Self: Sized,
    {
        super::Owner::Module(self.module)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamType {
    pub value: Value,
    pub depth: usize,
}

#[derive(Debug)]
pub struct Param {
    pub function: Id<Function>,
    pub element: Id<Element>,
    pub r#type: Option<ParamType>,
}

impl Managed for Param {
    type Local = ();

    type Onwer = Function;

    const NAME: &str = "Param";

    fn get_local(&self) -> &UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_local_mut(&mut self) -> &mut UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_owner(&self) -> super::Owner<Self::Onwer>
    where
        Self: Sized,
    {
        super::Owner::Managed(self.function)
    }
}
