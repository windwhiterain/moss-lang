use std::{fs, path::PathBuf};

use slotmap::new_key_type;
use tree_sitter::Parser;

use crate::{interpreter::{InterpreterLike, module::ModuleId}, utils::moss};
pub type Tree = type_sitter::Tree<moss::SourceFile<'static>>;

pub struct File {
    pub text: String,
    pub parser: Parser,
    pub tree: Tree,
    pub is_module: Option<ModuleId>,
    pub path: PathBuf,
}

new_key_type! {pub struct FileId;}

impl File {
    pub fn new(path: PathBuf,interpreter:&impl InterpreterLike) -> Self {
        let text = fs::read_to_string(interpreter.get_worksapce_path().join(&path)).unwrap();
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_moss::LANGUAGE.into())
            .unwrap();
        let tree = Tree::wrap(parser.parse(&text, None).unwrap());
        Self {
            text,
            parser,
            tree,
            is_module: None,
            path,
        }
    }
    pub fn update(&mut self,interpreter:&impl InterpreterLike) {
        self.text = fs::read_to_string(interpreter.get_worksapce_path().join(&self.path)).unwrap();
        self.tree = Tree::wrap(self.parser.parse(&self.text, None).unwrap());
        self.is_module = None;
    }
}
