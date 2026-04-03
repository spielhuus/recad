use sexp::{SexpTree, parser::SexpParser, SexpValueExt};
use types::{constants::el, error::RecadError};

use crate::symbols::LibrarySymbol;


///implement the symbol library.
pub struct SymbolLibrary {
    pub pathlist: Vec<std::path::PathBuf>,
}

impl SymbolLibrary {
    ///Load a symbol from the symbol library, the name is the combination
    ///of the filename of the library and the symbol name.
    pub fn load(&self, name: &str) -> Result<LibrarySymbol, RecadError> {
        let t: Vec<&str> = name.split(':').collect();
        for path in &self.pathlist {
            let filename = path.join(format!("{}.kicad_sym", t[0]));
            if let Ok(doc) = SexpParser::load(&filename) {
                spdlog::debug!("Load File: {:?}", filename);
                if let Ok(tree) = SexpTree::from(doc.iter()) {
                    for node in tree.root().query(el::SYMBOL) {
                        let sym_name: String = node.require_get(0)?;
                        if sym_name == t[1] {
                            let mut node: LibrarySymbol = LibrarySymbol::try_from(node)?;
                            node.lib_id = format!("{}:{}", t[0], t[1]);

                            if let Some(extends) = &node.extends {
                                if let Ok(mut ext_sym) =
                                    self.load(&format!("{}:{}", t.first().unwrap(), extends))
                                {
                                    for p in ext_sym.props.iter_mut() {
                                        for node_prp in &node.props {
                                            if p.key == node_prp.key {
                                                p.value.clone_from(&node_prp.value);
                                            }
                                        }
                                    }
                                    //ext_sym.props.clone_from(&node.props);
                                    ext_sym.lib_id = format!("{}:{}", t[0], t[1]);
                                    return Ok(ext_sym);
                                } else {
                                    return Err(RecadError::Schema(format!(
                                        "unable to find extend symbol {}",
                                        extends
                                    )));
                                }
                            }

                            return Ok(node);
                        }
                    }
                }
            }
        }
        Err(RecadError::Schema(format!(
            "can not find library: {}",
            name
        )))
    }
}
