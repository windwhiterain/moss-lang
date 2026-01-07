pub mod moss {
    pub mod moss_gen;
    pub use moss_gen::*;
    use type_sitter::HasChild;
    pub type ValueChild<'t> = <moss_gen::Value<'t> as HasChild<'t>>::Child;
    pub type BuiltinChild<'t> = <moss_gen::Builtin<'t> as HasChild<'t>>::Child;
    pub type StringContentChild<'t> = <moss_gen::StringContent<'t> as HasChild<'t>>::Child;
}
