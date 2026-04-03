use std::{fs, path::Path, str::CharIndices};

use crate::RecadError;

use super::{IntState, State};

///Parse sexp document.
pub struct SexpParser {
    filename: Option<std::path::PathBuf>,
    content: String,
}

impl SexpParser {
    #[allow(dead_code)]
    pub fn from(content: String) -> Self {
        Self {
            content,
            filename: None,
        }
    }

    ///Load the SEXP tree into memory.
    pub fn load(filename: &Path) -> Result<Self, RecadError> {
        match fs::read_to_string(filename) {
            Ok(content) => Ok(Self {
                content,
                filename: Some(filename.into()),
            }),
            Err(err) => Err(RecadError::Io(format!(
                "Unable to read file: {} ({})",
                filename.to_str().unwrap(),
                err
            ))),
        }
    }
    pub fn iter(&self) -> SexpIter<'_> {
        SexpIter::new(&self.content)
    }

    pub fn get_error(self, error: RecadError) -> RecadError {
        if let RecadError::Sexp { line, col, msg } = error {
            let path = if let Some(path) = self.filename {
                path.to_str().expect("get filename as string").to_string()
            } else {
                String::from("none")
            };
            RecadError::Sexp {
                line,
                col,
                msg: format!(
                    "{}:{}:{} Error: {}\n{}\n{}^\n",
                    path,
                    line,
                    col,
                    msg,
                    self.content.lines().nth(line.saturating_sub(1)).unwrap_or("").replace("\t", "  "),
                    " ".repeat(col)
                ),
            }
        } else {
            error
        }
    }
}

///Sexp Iterator,
pub struct SexpIter<'a> {
    content: &'a str,
    chars: CharIndices<'a>,
    start_index: usize,
    int_state: IntState,
    current_line: usize,
    current_column: usize,
    token_line: usize,
    token_column: usize,
}

impl<'a> SexpIter<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            content,
            chars: content.char_indices(),
            start_index: 0,
            int_state: IntState::NotStarted,
            current_line: 1,
            current_column: 1,
            token_line: 1,
            token_column: 1,
        }
    }
    // TODO Seek to the next sibling of the current node.
    // pub fn next_sibling(&mut self) -> Option<State<'a>> {
    //     let mut count: usize = 1;
    //     loop {
    //         if let Some(indice) = self.chars.next() {
    //             match indice.1 {
    //                 '(' => {
    //                     count += 1;
    //                 }
    //                 ')' => {
    //                     count -= 1;
    //                     if count == 0 {
    //                         self.int_state = IntState::NotStarted;
    //                         return self.next();
    //                     }
    //                 }
    //                 '\"' => {
    //                     let mut last_char = '\0';
    //                     loop {
    //                         // collect the characters to the next quote
    //                         if let Some(ch) = self.chars.next() {
    //                             if ch.1 == '"' && last_char != '\\' {
    //                                 break;
    //                             }
    //                             last_char = ch.1;
    //                         }
    //                     }
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    // }
}

impl<'a> Iterator for SexpIter<'a> {
    type Item = State<'a>;
    ///Get the next node.
    fn next(&mut self) -> Option<Self::Item> {
        if let IntState::BeforeEndSymbol(line, col) = self.int_state {
            self.int_state = IntState::Values;
            return Some(State::EndSymbol(line, col));
        }
        while let Some(indice) = self.chars.next() {
            let ch = indice.1;
            let ch_line = self.current_line;
            let ch_col = self.current_column;

            if ch == '\n' {
                self.current_line += 1;
                self.current_column = 1;
            } else {
                self.current_column += 1;
            }

            match self.int_state {
                IntState::NotStarted => {
                    if ch == '(' {
                        self.start_index = indice.0 + 1;
                        self.int_state = IntState::Symbol;
                        self.token_line = ch_line;
                        self.token_column = ch_col + 1;
                    }
                }
                IntState::Symbol => {
                    if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == ')' {
                        let name = &self.content[self.start_index..indice.0];
                        self.start_index = indice.0 + 1;
                        let line = self.token_line;
                        let col = self.token_column;
                        self.int_state = if ch == ')' {
                            IntState::BeforeEndSymbol(ch_line, ch_col)
                        } else {
                            IntState::Values
                        };
                        return Some(State::StartSymbol(name, line, col));
                    }
                }
                IntState::Values => {
                    if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == ')' {
                        if indice.0 - self.start_index > 0 {
                            let value = &self.content[self.start_index..indice.0];
                            self.start_index = indice.0 + 1;
                            let line = self.token_line;
                            let col = self.token_column;
                            self.int_state = if ch == ')' {
                                IntState::BeforeEndSymbol(ch_line, ch_col)
                            } else {
                                IntState::Values
                            };
                            return Some(State::Values(value, line, col));
                        }
                        self.start_index = indice.0 + 1;
                        if ch == ')' {
                            return Some(State::EndSymbol(ch_line, ch_col));
                        }
                    } else if ch == '(' {
                        self.start_index = indice.0 + 1;
                        self.int_state = IntState::Symbol;
                        self.token_line = ch_line;
                        self.token_column = ch_col + 1;
                    } else if ch == '"' {
                        self.start_index = indice.0 + 1;
                        let line = ch_line;
                        let col = ch_col + 1;
                        // collect the characters to the next quote
                        let mut escaped = false;
                        for ch_inner in self.chars.by_ref() {
                            let inner_ch = ch_inner.1;
                            if inner_ch == '\n' {
                                self.current_line += 1;
                                self.current_column = 1;
                            } else {
                                self.current_column += 1;
                            }
                            if inner_ch == '"' && !escaped {
                                let value = &self.content[self.start_index..ch_inner.0];
                                self.start_index = ch_inner.0 + 1;
                                self.int_state = IntState::Values;
                                return Some(State::Text(value, line, col));
                            }
                            escaped = if inner_ch == '\\' { !escaped } else { false };
                        }
                    } else if indice.0 == self.start_index {
                        // start of a new value token
                        self.token_line = ch_line;
                        self.token_column = ch_col;
                    }
                }
                IntState::BeforeEndSymbol(_, _) => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::SexpParser, State};

    #[test]
    fn check_index() {
        let doc = SexpParser::from(String::from(
            r#"(node value1 value2 "value 3" "value 4" "" "value \"four\"" endval)"#,
        ));
        let mut iter = doc.iter();

        assert_eq!(iter.next(), Some(State::StartSymbol("node", 1, 2)));
        assert_eq!(iter.next(), Some(State::Values("value1", 1, 7)));
        assert_eq!(iter.next(), Some(State::Values("value2", 1, 14)));
        assert_eq!(iter.next(), Some(State::Text("value 3", 1, 22)));
        assert_eq!(iter.next(), Some(State::Text("value 4", 1, 32)));
        assert_eq!(iter.next(), Some(State::Text("", 1, 42)));
        assert_eq!(iter.next(), Some(State::Text(r#"value \"four\""#, 1, 45)));
        assert_eq!(iter.next(), Some(State::Values("endval", 1, 61)));
        assert_eq!(iter.next(), Some(State::EndSymbol(1, 67)));
    }

    #[test]
    fn simple_content() {
        let doc = SexpParser::from(String::from(
            r#"(node value1 value2 "value 3" "value 4" "" "value \"four\"" endval)"#,
        ));
        let mut node_name = String::new();
        let mut values = String::new();
        let mut texts = String::new();
        let mut count = 0;
        for state in doc.iter() {
            match state {
                State::StartSymbol(name, _, _) => {
                    node_name = name.to_string();
                    count += 1;
                }
                State::EndSymbol(_, _) => {
                    count -= 1;
                }
                State::Values(value, _, _) => {
                    values += value;
                }
                State::Text(value, _, _) => {
                    texts += value;
                }
            }
        }
        assert_eq!("node", node_name);
        assert_eq!(values, "value1value2endval");
        assert_eq!(texts, r#"value 3value 4value \"four\""#);
        assert_eq!(count, 0);
    }
    #[test]
    fn next_sub_symbol() {
        let doc = SexpParser::from(String::from("(node value1 (node2))"));
        let mut iter = doc.iter();

        assert!(matches!(
            iter.next(),
            Some(State::StartSymbol("node", 1, _))
        ));
        assert!(matches!(iter.next(), Some(State::Values("value1", 1, _))));
        assert!(matches!(
            iter.next(),
            Some(State::StartSymbol("node2", 1, _))
        ));
    }

    #[test]
    fn next_sub_symbol_values() {
        let doc = SexpParser::from(String::from("(node value1 (node2 value2))"));
        let mut count = 0;
        let mut ends = 0;
        let mut iter = doc.iter();
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("value2", *value);
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            ends -= 1;
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            ends -= 1;
        }
        assert_eq!(count, 4);
        assert_eq!(ends, 0);
    }
    #[test]
    fn next_sub_symbol_text() {
        let doc = SexpParser::from(String::from("(node value1 (node2 \"value 2\"))"));
        let mut count = 0;
        let mut ends = 0;
        let mut iter = doc.iter();
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Text(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("value 2", *value);
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            ends -= 1;
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            ends -= 1;
        }
        assert_eq!(count, 4);
        assert_eq!(ends, 0);
    }
    #[test]
    fn next_sub_symbol_text_escaped() {
        let doc = SexpParser::from(String::from(r#"(node value1 (node2 "value \"2\""))"#));
        let mut count = 0;
        let mut iter = doc.iter();
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Text(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!(r#"value \"2\""#, *value);
        }
        assert_eq!(count, 4);
    }
    #[test]
    fn next_sub_symbol_line_breaks() {
        let doc = SexpParser::from(String::from("(node value1\n(node2 \"value 2\"\n)\n)"));
        let mut count = 0;
        let mut iter = doc.iter();
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Text(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("value 2", *value);
        }
        assert_eq!(count, 4);
    }
    #[test]
    fn parse_stroke() {
        let doc = SexpParser::from(String::from(
            "(stroke (width 0) (type default) (color 0 0 0 0))",
        ));
        let mut count = 0;
        let mut ends = 0;
        let mut iter = doc.iter();

        use types::constants::el;

        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("stroke", *name);
        }
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("width", *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("type", *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("default", *value);
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        if let Some(State::StartSymbol(name, _, _)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!(el::COLOR, *name);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::Values(value, _, _)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        if let Some(State::EndSymbol(_, _)) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        assert_eq!(iter.next(), None);
        assert_eq!(count, 14);
        assert_eq!(ends, 0);
    }
}

#[test]
fn parse_stroke_exact_positions() {
    let doc = SexpParser::from(String::from(
        "(stroke (width 0) (type default) (color 0 0 0 0))",
    ));
    let mut iter = doc.iter();

    assert_eq!(iter.next(), Some(State::StartSymbol("stroke", 1, 2)));
    assert_eq!(iter.next(), Some(State::StartSymbol("width", 1, 10)));
    assert_eq!(iter.next(), Some(State::Values("0", 1, 16)));
    assert_eq!(iter.next(), Some(State::EndSymbol(1, 17)));
    assert_eq!(iter.next(), Some(State::StartSymbol("type", 1, 20)));
    assert_eq!(iter.next(), Some(State::Values("default", 1, 25)));
    assert_eq!(iter.next(), Some(State::EndSymbol(1, 32)));
    assert_eq!(iter.next(), Some(State::StartSymbol("color", 1, 35)));
    assert_eq!(iter.next(), Some(State::Values("0", 1, 41)));
    assert_eq!(iter.next(), Some(State::Values("0", 1, 43)));
    assert_eq!(iter.next(), Some(State::Values("0", 1, 45)));
    assert_eq!(iter.next(), Some(State::Values("0", 1, 47)));
    assert_eq!(iter.next(), Some(State::EndSymbol(1, 48)));
    assert_eq!(iter.next(), Some(State::EndSymbol(1, 49)));
    assert_eq!(iter.next(), None);
}
