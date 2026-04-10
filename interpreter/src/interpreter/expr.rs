use enum_extract_macro::EnumExtract;

use crate::{
    interpreter::{Id, element::Element, function::Function, scope::Scope, value::ValueStorage},
    utils::concurrent_string_interner::StringId,
};

pub trait HasRef {
    fn map_ref(&mut self, _map: impl FnMut(Id<Element>) -> Id<Element>) {}
    fn iter_ref(&self, _map: impl FnMut(Id<Element>)) {}
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct Ref {
    pub element: Id<Element>,
}

impl HasRef for Ref {
    fn map_ref(&mut self, mut map: impl FnMut(Id<Element>) -> Id<Element>) {
        self.element = map(self.element);
    }

    fn iter_ref(&self, mut map: impl FnMut(Id<Element>)) {
        map(self.element);
    }
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct FunctionBody {
    pub function: Id<Function>,
}

impl HasRef for FunctionBody {}

impl HasRef for ValueStorage {
    fn map_ref(&mut self, mut map: impl FnMut(Id<Element>) -> Id<Element>) {
        match self {
            ValueStorage::Element(element) => element.0 = map(element.0),
            _ => (),
        }
    }

    fn iter_ref(&self, mut map: impl FnMut(Id<Element>)) {
        match self {
            ValueStorage::Element(element) => map(element.0),
            _ => (),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompleteScope(pub Id<Scope>);
impl HasRef for CompleteScope {}

#[derive(Clone, Copy, Debug)]
pub struct EffectiveScope(pub Id<Scope>);
impl HasRef for EffectiveScope {}

#[derive(Clone, Copy, Debug, EnumExtract)]
pub enum Expr {
    Ref(Ref),
    Find(Find),
    Call(Call),
    FunctionBody(FunctionBody),
    CompleteScope(CompleteScope),
    EffectiveScope(EffectiveScope),
    Value(ValueStorage),
}

impl HasRef for Expr {
    fn map_ref(&mut self, map: impl FnMut(Id<Element>) -> Id<Element>) {
        match self {
            Expr::Ref(value) => value.map_ref(map),
            Expr::Find(value) => value.map_ref(map),
            Expr::Call(value) => value.map_ref(map),
            Expr::FunctionBody(value) => value.map_ref(map),
            Expr::CompleteScope(value) => value.map_ref(map),
            Expr::EffectiveScope(value) => value.map_ref(map),
            Expr::Value(value) => value.map_ref(map),
        }
    }

    fn iter_ref(&self, map: impl FnMut(Id<Element>)) {
        match self {
            Expr::Ref(value) => value.iter_ref(map),
            Expr::Find(value) => value.iter_ref(map),
            Expr::Call(value) => value.iter_ref(map),
            Expr::FunctionBody(value) => value.iter_ref(map),
            Expr::CompleteScope(value) => value.iter_ref(map),
            Expr::EffectiveScope(value) => value.iter_ref(map),
            Expr::Value(value) => value.iter_ref(map),
        }
    }
}
