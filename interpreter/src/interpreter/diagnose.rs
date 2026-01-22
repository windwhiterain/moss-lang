use type_sitter::UntypedNode;

use crate::{interpreter::value::Value, utils::concurrent_string_interner::StringId};

#[derive(Clone, Debug)]
pub enum Diagnostic {
    GrammarError {
        source: UntypedNode<'static>,
    },
    RedundantElementKey {
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
    PathError {
        source: UntypedNode<'static>,
    },
    StringEscapeError {
        source: UntypedNode<'static>,
    },
    Custom {
        source: UntypedNode<'static>,
        text: StringId,
    },
}
