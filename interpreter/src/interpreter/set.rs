use crate::{
    interpreter::{Id, Managed, Owner, element::Element, module::ModuleId},
    utils::unsafe_cell::UnsafeCell,
};

#[derive(Debug)]
pub struct Set {
    pub elements: Vec<Id<Element>>,
    pub owner: Owner,
}

impl Managed for Set {
    const NAME: &str = "Set";

    type Local = ();

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
