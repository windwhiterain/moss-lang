use std::fmt::Display;

use crate::{
    interpreter::{InterpreterLike, value::Value},
    utils::{
        concurrent_string_interner::StringId,
        contexted::{Contexted, WithContext},
    },
};

#[derive(Clone, Debug)]
pub enum Diagnostic {
    GrammarError {},
    RedundantElementKey {},
    FailedFindElement {},
    FialedFindElementOrPrivateElement {},
    CanNotFindIn { value: Value },
    CanNotCallOn { value: Value },
    StringEscapeError {},
    Custom { text: StringId },
}

impl Diagnostic {
    pub fn is_key(&self) -> bool {
        match self {
            Diagnostic::RedundantElementKey {} => true,
            _ => false,
        }
    }
}

impl<'a, IP: InterpreterLike> Display for Contexted<'a, Diagnostic, IP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value {
            Diagnostic::GrammarError {} => write!(f, "grammar error"),
            Diagnostic::RedundantElementKey {} => write!(f, "redundant element key"),
            Diagnostic::FailedFindElement {} => write!(f, "failed find element"),
            Diagnostic::FialedFindElementOrPrivateElement {} => {
                write!(f, "failed find element or private element")
            }
            Diagnostic::CanNotFindIn { value } => {
                write!(f, "can not find in {}", value.with_ctx(self.ctx))
            }
            Diagnostic::CanNotCallOn { value } => {
                write!(f, "caan not call on {}", value.with_ctx(self.ctx))
            }
            Diagnostic::StringEscapeError {} => write!(f, "string escape errorr"),
            Diagnostic::Custom { text } => write!(f, "{}", &*self.ctx.id2str(*text)),
        }
    }
}
