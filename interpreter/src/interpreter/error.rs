use std::fmt::Display;

use crate::{
    interpreter::{InterpreterLike, Location, Managed, value::ValueStorage},
    utils::{
        concurrent_string_interner::StringId,
        contexted::{Contexted, WithContext},
    },
};

#[derive(Clone, Copy, Debug)]
pub struct Error {
    pub kind: Kind,
    pub location: Location,
}

impl Managed for Error {
    type Local = ();

    const NAME: &str = "Error";

    fn get_local(&self) -> &crate::utils::unsafe_cell::UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_local_mut(&mut self) -> &mut crate::utils::unsafe_cell::UnsafeCell<Self::Local> {
        unimplemented!()
    }

    fn get_module<IP: InterpreterLike>(&self, _ip: &IP) -> super::module::ModuleId
    where
        Self: Sized,
    {
        unimplemented!()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    GrammarError {},
    RedundantElementKey {},
    FailedFindElement {},
    FialedFindElementOrPrivateElement {},
    CanNotFindIn { value: ValueStorage },
    CanNotCallOn { value: ValueStorage },
    StringEscapeError {},
    Custom { text: StringId },
}

impl Kind {
    pub fn is_key(&self) -> bool {
        match self {
            Kind::RedundantElementKey {} => true,
            _ => false,
        }
    }
}

impl<'a, IP: InterpreterLike> Display for Contexted<'a, Kind, IP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value {
            Kind::GrammarError {} => write!(f, "grammar error"),
            Kind::RedundantElementKey {} => write!(f, "redundant element key"),
            Kind::FailedFindElement {} => write!(f, "failed find element"),
            Kind::FialedFindElementOrPrivateElement {} => {
                write!(f, "failed find element or private element")
            }
            Kind::CanNotFindIn { value } => {
                write!(f, "can not find in {}", value.with_ctx(self.ctx))
            }
            Kind::CanNotCallOn { value } => {
                write!(f, "caan not call on {}", value.with_ctx(self.ctx))
            }
            Kind::StringEscapeError {} => write!(f, "string escape errorr"),
            Kind::Custom { text } => write!(f, "{}", &*self.ctx.id2str(*text)),
        }
    }
}
