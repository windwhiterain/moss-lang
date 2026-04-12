use std::{fs, path::PathBuf};

use slotmap::new_key_type;
use moss_parser::Parser;
use moss_parser::Node;

use crate::{
    interpreter::{InterpreterLike, module::ModuleId},
};
pub use moss_parser::Tree;

pub struct File {
    pub text: String,
    pub parser: Parser,
    pub tree: Tree,
    pub module: Option<ModuleId>,
    pub path: PathBuf,
}

new_key_type! {pub struct FileId;}

impl File {
    pub fn new(path: PathBuf, interpreter: &impl InterpreterLike) -> Self {
        let text = fs::read_to_string(interpreter.get_worksapce_path().join(&path)).unwrap();
        let mut parser = Parser::new();
        parser
            .set_language(&moss_parser::LANGUAGE.into())
            .unwrap();
        let tree = Tree::wrap(parser.parse(&text, None).unwrap());
        log::error!("syntax {}:\n{}", path.display(), tree.root_node().to_sexp());
        Self {
            text,
            parser,
            tree,
            module: None,
            path,
        }
    }
    pub fn update(&mut self, interpreter: &impl InterpreterLike) {
        self.text = fs::read_to_string(interpreter.get_worksapce_path().join(&self.path)).unwrap();
        self.tree = Tree::wrap(self.parser.parse(&self.text, None).unwrap());
        log::error!("sytax:\n{}", self.tree.root_node().to_sexp());
        self.module = None;
    }
}
