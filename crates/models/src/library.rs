use sexp::{parser::SexpParser, SexpTree, SexpValueExt};
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use types::gr::Pos;

    use crate::schema::{Property, Schema, SchemaItem, Symbol};

    use super::*;

    #[test]
    fn test_resistor() {
        let library = SymbolLibrary {
            pathlist: vec![PathBuf::from("/usr/share/kicad/symbols")],
        };
        let res = library.load("Device:R").unwrap();
        assert_eq!(res.lib_id, "Device:R");
        for prop in &res.props {
            if ["Reference", "Value"].contains(&prop.key.as_str()) {
                assert!(prop.visible());
            } else if [
                "Footprint",
                "Datasheet",
                "Description",
                "ki_keywords",
                "ki_description",
                "ki_fp_filters",
            ]
            .contains(&prop.key.as_str())
            {
                assert!(!prop.visible());
            } else {
                todo!("unknown property: {}", prop.key);
            }
        }
    }

    #[test]
    fn test_write_resistor() {
        let library = SymbolLibrary {
            pathlist: vec![PathBuf::from("/usr/share/kicad/symbols")],
        };
        let res = library.load("Device:R").unwrap();
        assert_eq!(res.lib_id, "Device:R");

        let mut schema = Schema::new("test opamp", None);
        schema.library_symbols.push(res.clone());
        let symbol = Symbol {
            lib_id: "Device:R".to_string(),
            unit: 1,
            pos: Pos {
                x: 25.4,
                y: 25.4,
                angle: 0.0,
            },
            uuid: crate::uuid!(),
            props: res
                .props
                .iter()
                .map(|p| {
                    let mut prop = p.clone();
                    if prop.key == "Reference" {
                        prop.value = "R1".to_string();
                        prop.pos = Pos {
                            x: 28.0,
                            y: 25.0,
                            angle: 0.0,
                        };
                    } else if prop.key == "Value" {
                        prop.value = "100k".to_string();
                        prop.pos = Pos {
                            x: 28.0,
                            y: 27.0,
                            angle: 0.0,
                        };
                    }
                    prop
                })
                .collect(),
            ..Default::default()
        };

        schema.items.push(SchemaItem::Symbol(symbol));
        let mut file = std::fs::File::create("../../target/resistor_write.kicad_sch").unwrap();
        schema.write(&mut file).unwrap();

        //load the file again
        let schema = Schema::load(&PathBuf::from("../../target/resistor_write.kicad_sch"), None).unwrap();
        assert_eq!(schema.library_symbols.len(), 1);
        assert_eq!(schema.items.len(), 1);

        let symbol: Vec<Option<&Symbol>> = schema
            .items
            .iter()
            .map(|i| {
                if let SchemaItem::Symbol(s) = i {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();

        let symbol = symbol.first().unwrap().unwrap();
        assert_eq!(symbol.props.len(), 7);

        let prop: Vec<&Property> = symbol.props.iter().filter(|p| {
            p.key == "Datasheet"
        }).collect();   
        let prop = prop.first().unwrap();
        assert_eq!(prop.key, "Datasheet");
        assert!(!prop.visible());

    }
}
