use crate::{
    interpreter::{Id, Managed, element::Element, module::ModuleId},
    utils::unsafe_cell::UnsafeCell,
};

#[derive(Debug)]
pub struct Set {
    pub elements: Vec<Id<Element>>,
    pub module: ModuleId,
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

    type Onwer = Self;

    fn get_owner(&self) -> super::Owner<Self::Onwer>
    where
        Self: Sized,
    {
        super::Owner::Module(self.module)
    }
}
