use crate::Error;

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
///
///The sruct us used by the sexp macro.
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
    pub fn push(&mut self, name: &str) {
        self.level += 1;
        self.nodes.push(BuilderState::StartSymbol(name.to_string()));
    }
    pub fn end(&mut self) {
        self.level -= 1;
        self.nodes.push(BuilderState::EndSymbol);
    }
    pub fn value(&mut self, name: &str) {
        self.nodes.push(BuilderState::Values(name.to_string()));
    }
    pub fn text(&mut self, name: &str) {
        self.nodes.push(BuilderState::Text(name.to_string()));
    }
    ///return a SexpTree.
    pub fn sexp(&self) -> Result<SexpTree, Error> {
        let mut iter = self.nodes.iter();
        let mut stack: Vec<(String, Sexp)> = Vec::new();
        if let Some(BuilderState::StartSymbol(name)) = iter.next() {
            stack.push((name.to_string(), Sexp::from(name.to_string())));
        } else {
            return Err(Error(
                String::from("Document does not start with a start symbol."),
                String::from("sexp"),
            ));
        };
        loop {
            match iter.next() {
                Some(BuilderState::Values(value)) => {
                    let len = stack.len();
                    if let Some((_, parent)) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Value(value.to_string()));
                    }
                }
                Some(BuilderState::Text(value)) => {
                    let len = stack.len();
                    if let Some((_, parent)) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Text(value.to_string()));
                    }
                }
                Some(BuilderState::EndSymbol) => {
                    let len = stack.len();
                    if len > 1 {
                        let (_n, i) = stack.pop().unwrap();
                        if let Some((_, parent)) = stack.get_mut(len - 2) {
                            parent.nodes.push(SexpAtom::Node(i));
                        }
                    }
                }
                Some(BuilderState::StartSymbol(name)) => {
                    stack.push((name.to_string(), Sexp::from(name.to_string())));
                }
                None => break,
            }
        }
        let (_n, i) = stack.pop().unwrap();
        Ok(SexpTree { tree: i })
    }
}
