use crate::interpreter::element::InModuleElementId;
use crate::interpreter::value::TypedValue;
use crate::utils::type_key::Vec as KeyVec;
pub struct Element {
    pub value: TypedValue,
    pub resolved: bool,
}
pub struct Function {
    pub elements: KeyVec<InModuleElementId, Element>,
}
