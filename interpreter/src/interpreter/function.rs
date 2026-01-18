use crate::interpreter::{Id, Managed, element::Element, module::ModuleId, scope::Scope};

pub struct Function{
    pub scope: Id<Scope>,
    pub r#in: Id<Element>,
    pub module: ModuleId,
}

impl Managed for Function{
    type Local = ();

    type Onwer = Function;

    const NAME: &str = "Function";

    fn get_local(&self) -> &std::cell::UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_local_mut(&mut self) -> &mut std::cell::UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_owner(&self) -> super::Owner<Self::Onwer>
    where
        Self: Sized {
        super::Owner::Module(self.module)
    }
}