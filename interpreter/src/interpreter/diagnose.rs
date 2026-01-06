use type_sitter::UntypedNode;

use crate::interpreter::Value;

#[derive(Clone)]
pub enum Diagnostic {
    GrammarError {
        source: UntypedNode<'static>,
    },
    ElementKeyRedundancy {
        source: UntypedNode<'static>,
    },
    FailedFindElement {
        source: UntypedNode<'static>,
    },
    FialedFindElementOrPrivateElement {
        source: UntypedNode<'static>,
    },
    CanNotFindIn {
        source: UntypedNode<'static>,
        value: Value,
    },
    CanNotCallOn {
        source: UntypedNode<'static>,
        value: Value,
    },
}
