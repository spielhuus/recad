use std::borrow::Cow;

use crate::RecadError;

use super::{Sexp, SexpAtom, SexpTree};

/// internal state of the sexp builder.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BuilderState {
    StartSymbol(String),
    EndSymbol,
    Values(String),
    Text(String),
}

/// utility to build a sexp document.
pub struct Builder {
    pub nodes: Vec<BuilderState>,
    pub level: usize,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            level: 0,
        }
    }

    pub fn push(&mut self, name: impl std::fmt::Display) {
        self.level += 1;
        self.nodes.push(BuilderState::StartSymbol(name.to_string()));
    }

    pub fn end(&mut self) {
        self.level -= 1;
        self.nodes.push(BuilderState::EndSymbol);
    }

    pub fn value(&mut self, value: impl std::fmt::Display) {
        self.nodes.push(BuilderState::Values(value.to_string()));
    }

    pub fn text(&mut self, text: impl std::fmt::Display) {
        self.nodes.push(BuilderState::Text(text.to_string()));
    }

    ///return a SexpTree.
    pub fn sexp(&self) -> Result<SexpTree<'_>, RecadError> {
        let mut iter = self.nodes.iter();
        let mut stack: Vec<Sexp> = Vec::new();
        if let Some(BuilderState::StartSymbol(name)) = iter.next() {
            stack.push(Sexp::from(Cow::Owned(name.to_string()), 0, 0));
        } else {
            return Err(RecadError::Writer(String::from(
                "Document does not start with a start symbol.",
            )));
        };
        loop {
            match iter.next() {
                Some(BuilderState::Values(value)) => {
                    let len = stack.len();
                    if let Some(parent) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Value(Cow::Owned(value.to_string())));
                    }
                }
                Some(BuilderState::Text(value)) => {
                    let len = stack.len();
                    if let Some(parent) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Text(Cow::Owned(value.to_string())));
                    }
                }
                Some(BuilderState::EndSymbol) => {
                    let len = stack.len();
                    if len > 1 {
                        let i = stack.pop().unwrap();
                        if let Some(parent) = stack.get_mut(len - 2) {
                            parent.nodes.push(SexpAtom::Node(i));
                        }
                    }
                }
                Some(BuilderState::StartSymbol(name)) => {
                    stack.push(Sexp::from(Cow::Owned(name.to_string()), 0, 0));
                }
                None => break,
            }
        }
        let i = stack.pop().unwrap();
        Ok(SexpTree { tree: i })
    }
}
