use std::{fs, path::Path, str::CharIndices};

use crate::Error;

use super::{IntState, State};

///Parse sexo document.
pub struct SexpParser {
    content: String,
}

impl SexpParser {
    pub fn from(content: String) -> Self {
        Self { content }
    }
    ///Load the SEXP tree into memory.
    pub fn load(filename: &Path) -> Result<Self, Error> {
        match fs::read_to_string(filename) {
            Ok(content) => Ok(Self::from(content)),
            Err(err) => Err(Error(filename.to_str().unwrap().to_string(), err.to_string())),
        }
    }
    pub fn iter(&self) -> SexpIter<'_> {
        SexpIter::new(&self.content)
    }
}

///Sexp Iterator,
pub struct SexpIter<'a> {
    content: &'a String,
    chars: CharIndices<'a>,
    start_index: usize,
    int_state: IntState,
}

impl<'a> SexpIter<'a> {
    fn new(content: &'a String) -> Self {
        Self {
            content,
            chars: content.char_indices(),
            start_index: 0,
            int_state: IntState::NotStarted,
        }
    }
    ///Seek to the next siebling of the current node.
    pub fn next_siebling(&mut self) -> Option<State<'a>> {
        let mut count: usize = 1;
        loop {
            if let Some(indice) = self.chars.next() {
                match indice.1 {
                    '(' => {
                        count += 1;
                    }
                    ')' => {
                        count -= 1;
                        if count == 0 {
                            self.int_state = IntState::NotStarted;
                            return self.next();
                        }
                    }
                    '\"' => {
                        let mut last_char = '\0';
                        loop {
                            // collect the characters to the next quote
                            if let Some(ch) = self.chars.next() {
                                if ch.1 == '"' && last_char != '\\' {
                                    break;
                                }
                                last_char = ch.1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl<'a> Iterator for SexpIter<'a> {
    type Item = State<'a>;
    ///Get the next node.
    fn next(&mut self) -> Option<Self::Item> {
        if self.int_state == IntState::BeforeEndSymbol {
            self.int_state = IntState::Values;
            return Some(State::EndSymbol);
        }
        while let Some(indice) = self.chars.next() {
            match self.int_state {
                IntState::NotStarted => {
                    if indice.1 == '(' {
                        self.start_index = indice.0 + 1;
                        self.int_state = IntState::Symbol;
                    }
                }
                IntState::Symbol => {
                    if indice.1 == ' ' || indice.1 == '\n' || indice.1 == ')' {
                        let name = &self.content[self.start_index..indice.0];
                        self.start_index = indice.0 + 1;
                        self.int_state = if indice.1 == ')' {
                            IntState::BeforeEndSymbol
                        } else {
                            IntState::Values
                        };
                        return Some(State::StartSymbol(name));
                    }
                }
                IntState::Values => {
                    if indice.1 == ' ' || indice.1 == '\t' || indice.1 == '\n' || indice.1 == ')' {
                        if indice.0 - self.start_index > 0 {
                            let value = &self.content[self.start_index..indice.0];
                            self.start_index = indice.0 + 1;
                            self.int_state = if indice.1 == ')' {
                                IntState::BeforeEndSymbol
                            } else {
                                IntState::Values
                            };
                            return Some(State::Values(value));
                        }
                        self.start_index = indice.0 + 1;
                        if indice.1 == ')' {
                            return Some(State::EndSymbol);
                        }
                    } else if indice.1 == '(' {
                        self.start_index = indice.0 + 1;
                        self.int_state = IntState::Symbol;
                    } else if indice.1 == '"' {
                        let mut last_char = '\0';
                        self.start_index = indice.0 + 1;
                        loop {
                            // collect the characters to the next quote
                            if let Some(ch) = self.chars.next() {
                                if ch.1 == '"' && last_char != '\\' {
                                    let value = &self.content[self.start_index..ch.0];
                                    self.start_index = ch.0 + 1;
                                    self.int_state = if indice.1 == ')' {
                                        IntState::BeforeEndSymbol
                                    } else {
                                        IntState::Values
                                    };
                                    return Some(State::Text(value));
                                }
                                last_char = ch.1;
                            }
                        }
                    }
                }
                IntState::BeforeEndSymbol => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::sexp::{constants::el, parser::SexpParser, State};

    #[test]
    fn check_index() {
        let doc = SexpParser::from(String::from(
            r#"(node value1 value2 "value 3" "value 4" "" "value \"four\"" endval)"#,
        ));
        let mut iter = doc.iter();
        let state = iter.next();
        assert_eq!(state, Some(State::StartSymbol("node")));

        let state = iter.next();
        assert_eq!(state, Some(State::Values("value1")));

        let state = iter.next();
        assert_eq!(state, Some(State::Values("value2")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text("value 3")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text("value 4")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text("")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text(r#"value \"four\""#)));

        let state = iter.next();
        assert_eq!(state, Some(State::Values("endval")));

        let state = iter.next();
        assert_eq!(state, Some(State::EndSymbol));
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
                State::StartSymbol(name) => {
                    node_name = name.to_string();
                    count += 1;
                }
                State::EndSymbol => {
                    count -= 1;
                }
                State::Values(value) => {
                    values += value;
                }
                State::Text(value) => {
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
        let state = iter.next();
        assert_eq!(state, Some(State::StartSymbol("node")));

        let state = iter.next();
        assert_eq!(state, Some(State::Values("value1")));

        let state = iter.next();
        assert_eq!(state, Some(State::StartSymbol("node2")));
    }

    #[test]
    fn next_sub_symbol_values() {
        let doc = SexpParser::from(String::from("(node value1 (node2 value2))"));
        let mut count = 0;
        let mut ends = 0;
        let mut iter = doc.iter();
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("value2", *value);
        }
        if let Some(State::EndSymbol) = &iter.next() {
            ends -= 1;
        }
        if let Some(State::EndSymbol) = &iter.next() {
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
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Text(value)) = &iter.next() {
            count += 1;
            assert_eq!("value 2", *value);
        }
        if let Some(State::EndSymbol) = &iter.next() {
            ends -= 1;
        }
        if let Some(State::EndSymbol) = &iter.next() {
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
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Text(value)) = &iter.next() {
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
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            assert_eq!("node", *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("value1", *value);
        }
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            assert_eq!("node2", *name);
        }
        if let Some(State::Text(value)) = &iter.next() {
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
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("stroke", *name);
        }
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("width", *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::EndSymbol) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!("type", *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("default", *value);
        }
        if let Some(State::EndSymbol) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        if let Some(State::StartSymbol(name)) = &iter.next() {
            count += 1;
            ends += 1;
            assert_eq!(el::COLOR, *name);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::Values(value)) = &iter.next() {
            count += 1;
            assert_eq!("0", *value);
        }
        if let Some(State::EndSymbol) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        if let Some(State::EndSymbol) = &iter.next() {
            count += 1;
            ends -= 1;
        }
        assert_eq!(iter.next(), None);
        assert_eq!(count, 14);
        assert_eq!(ends, 0);
    }
}
