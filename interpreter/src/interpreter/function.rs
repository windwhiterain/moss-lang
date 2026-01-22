

use crate::{
    interpreter::{
        Id, Managed, element::{Element, ElementKey}, expr::Expr, module::ModuleId, scope::Scope, value::Value
    }, utils::{concurrent_string_interner::StringId, unsafe_cell::UnsafeCell},
};

#[derive(Debug)]
pub enum FunctionElementAuthored{
    Expr(Expr),
    Value(Value),
}

#[derive(Debug)]
pub struct FunctionElement {
    pub authored:FunctionElementAuthored,
    pub key: ElementKey,
}

#[derive(Debug)]
pub struct FunctionScope {
    pub elements:Vec<Id<Element>>,
}

#[derive(Debug)]
pub struct FunctionOptimized {
    pub elements: Vec<FunctionElement>,
    pub scopes: Vec<FunctionScope>,
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
                scopes:Default::default(),
                root_scope:None
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
pub struct Param{
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