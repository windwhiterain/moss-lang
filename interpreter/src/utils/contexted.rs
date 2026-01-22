pub struct Contexted<'a, T: ?Sized, Ctx: ?Sized> {
    pub value: &'a T,
    pub ctx: &'a Ctx,
}

pub trait WithContext {
    fn with_ctx<'a, Ctx: ?Sized>(&'a self, ctx: &'a Ctx) -> Contexted<'a, Self, Ctx> {
        Contexted { value: self, ctx }
    }
}

impl<T> WithContext for T {}
