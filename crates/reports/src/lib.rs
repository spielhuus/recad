pub mod bom;
pub mod drc;
pub mod erc;

use yaml_rust::Yaml;

/// BOM Item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomItem {
    pub amount: usize,
    pub references: Vec<String>,
    pub value: String,
    pub footprint: String,
    pub datasheet: String,
    pub description: String,
    pub mouser_nr: String,
}

impl From<Yaml> for BomItem {
    fn from(yaml: Yaml) -> Self {
        let mut value = String::new();
        let mut description = String::new();
        let mut footprint = String::new();
        let mut datasheet = String::new();
        let mut mouser_nr = String::new();
        if let Yaml::Hash(hash) = yaml {
            let key = hash.keys().next().unwrap();
            if let Yaml::String(key) = key {
                description = key.to_string();
            } else {
                panic!("part key is not a String.");
            }
            if let Yaml::Array(items) = hash.get(key).unwrap() {
                for item in items {
                    if let Yaml::Hash(value_hash) = item {
                        let k = value_hash.keys().next().unwrap();
                        let v = value_hash.get(k).unwrap();
                        if let (Yaml::String(k), Yaml::String(v)) = (k, v) {
                            if k == "value" {
                                value = v.to_string();
                            } else if k == "footprint" {
                                footprint = v.to_string();
                            } else if k == "datasheet" {
                                datasheet = v.to_string();
                            } else if k == "mouser" {
                                mouser_nr = v.to_string();
                            }
                        }
                    }
                }
            }
        }
        BomItem {
            amount: 0,
            references: vec![],
            value,
            footprint,
            datasheet,
            description,
            mouser_nr,
        }
    }
}
