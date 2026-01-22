use enum_extract_macro::EnumExtract;

use crate::{
    interpreter::{Id, element::Element, function::Function, value::Value},
    utils::concurrent_string_interner::StringId,
};

pub trait HasRef {
    fn map_ref(&mut self, _map: impl FnMut(Id<Element>) -> Id<Element>) {}
    fn iter_ref(&self, _map: impl FnMut(Id<Element>)) {}
}

#[derive(Clone, Debug)]
pub struct Find {
    pub target: Option<Id<Element>>,
    pub name: StringId,
    pub meta: bool,
}

impl HasRef for Find {
    fn map_ref(&mut self, mut map: impl FnMut(Id<Element>) -> Id<Element>) {
        if let Some(target) = self.target {
            self.target = Some(map(target));
        }
    }

    fn iter_ref(&self, mut map: impl FnMut(Id<Element>)) {
        if let Some(target) = self.target {
            map(target);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ref {
    pub element_id: Id<Element>,
}

impl HasRef for Ref {
    fn map_ref(&mut self, mut map: impl FnMut(Id<Element>) -> Id<Element>) {
        self.element_id = map(self.element_id);
    }

    fn iter_ref(&self, mut map: impl FnMut(Id<Element>)) {
        map(self.element_id);
    }
}

#[derive(Clone, Debug)]
pub struct Call {
    pub function: Id<Element>,
    pub param: Id<Element>,
}

impl HasRef for Call {
    fn map_ref(&mut self, mut map: impl FnMut(Id<Element>) -> Id<Element>) {
        self.function = map(self.function);
        self.param = map(self.param);
    }

    fn iter_ref(&self, mut map: impl FnMut(Id<Element>)) {
        map(self.param);
        map(self.param);
    }
}

#[derive(Clone, Debug)]
pub struct FunctionOptimize {
    pub function: Id<Function>,
}

impl HasRef for FunctionOptimize {}

impl HasRef for Value {
    fn map_ref(&mut self, mut map: impl FnMut(Id<Element>) -> Id<Element>) {
        match self {
            Value::Element(element) => element.0 = map(element.0),
            _ => (),
        }
    }

    fn iter_ref(&self, mut map: impl FnMut(Id<Element>)) {
        match self {
            Value::Element(element) => map(element.0),
            _ => (),
        }
    }
}

#[derive(Clone, Debug, EnumExtract)]
pub enum Expr {
    Ref(Ref),
    Find(Find),
    Call(Call),
    FunctionOptimize(FunctionOptimize),
    Value(Value),
}

impl HasRef for Expr {
    fn map_ref(&mut self, map: impl FnMut(Id<Element>) -> Id<Element>) {
        match self {
            Expr::Ref(value) => value.map_ref(map),
            Expr::Find(value) => value.map_ref(map),
            Expr::Call(value) => value.map_ref(map),
            Expr::FunctionOptimize(value) => value.map_ref(map),
            Expr::Value(value) => value.map_ref(map),
        }
    }

    fn iter_ref(&self, map: impl FnMut(Id<Element>)) {
        match self {
            Expr::Ref(value) => value.iter_ref(map),
            Expr::Find(value) => value.iter_ref(map),
            Expr::Call(value) => value.iter_ref(map),
            Expr::FunctionOptimize(value) => value.iter_ref(map),
            Expr::Value(value) => value.iter_ref(map),
        }
    }
}
