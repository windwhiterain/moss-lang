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
pub struct FunctionOptimized {
    pub elements: KeyVec<Id<Element>, FunctionElement>,
    pub scopes: KeyVec<Id<Scope>, FunctionScope>,
    pub root_scope: Option<Id<Scope>>,
}

pub const OPTIMIZED_PARAM: Id<Element> = Id::from_idx(usize::MAX);

#[derive(Debug)]
pub struct Function {
    pub scope: Id<Scope>,
    pub r#in: Id<Element>,
    pub module: ModuleId,
    pub complete: Id<Element>,
    pub optimized: UnsafeCell<FunctionOptimized>,
}

impl Function {
    pub fn new(
        scope: Id<Scope>,
        r#in: Id<Element>,
        module: ModuleId,
        complete: Id<Element>,
    ) -> Self {
        Self {
            scope,
            r#in,
            module,
            complete,
            optimized: UnsafeCell::new(FunctionOptimized {
                elements: Default::default(),
                scopes: Default::default(),
                root_scope: None,
            }),
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
