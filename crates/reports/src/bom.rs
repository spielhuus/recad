//! Create a BOM for the Schema.
//!
//! # Example:
//!
//! use recad_core::Schema;
//! use recad_core::reports::bom::bom;
//! use std::path::PathBuf;
//!
//! let schema = Schema::load("tests/summe/summe.kicad_sch").unwrap();
//! let result = bom(&schema, true, Some(PathBuf::from("files/partlist.yaml"))).unwrap();
//! println!("Items not found {:#?}", result.1);
//!

use types::{
    error::RecadError,
    constants::el,
};

use models::schema::{Schema, SchemaItem};
use std::{collections::HashMap, path::PathBuf};
use yaml_rust::{Yaml, YamlLoader};

use crate::BomItem;


/// Read and parse a partlist YAML file into BOM items.
///
/// # Arguments
///
/// * `partlist` - Path to the YAML file containing part information.
///
/// # Returns
///
/// A vector of `BomItem` structs parsed from the YAML file, or an error if the file cannot be read or parsed.
fn get_partlist(partlist: &str) -> Result<Vec<BomItem>, RecadError> {
    let content = match std::fs::read_to_string(partlist) {
        Ok(content) => content,
        Err(err) => {
            return Err(RecadError::Io(format!(
                "Unable to load partlist YAML file from: {} ({})",
                partlist, err
            )))
        }
    };
    let partlist = match YamlLoader::load_from_str(&content) {
        Ok(content) => content,
        Err(err) => {
            return Err(RecadError::Io(format!(
                "Unable to parse YAML content of file: {} ({})",
                partlist, err
            )))
        }
    };

    let mut bom: Vec<BomItem> = Vec::new();
    for items in partlist {
        if let Yaml::Array(items) = items {
            for item in items {
                bom.push(item.into());
            }
        }
    }
    Ok(bom)
}


/// Format a reference designator string by separating characters and numbers.
///
/// # Arguments
///
/// * `value` - The reference designator string (e.g., "R1", "C10").
///
/// # Returns
///
/// A formatted string with the numeric part padded to 4 digits with leading zeros.
///
/// # Example
///
///```ignore
/// reference("R1")    // Returns "R0001" 
/// reference("C10")   // Returns "C0010" 
/// ```
fn reference(value: &str) -> String {
    let mut reference_characters = String::new();
    let mut reference_numbers = String::new();
    for c in value.chars() {
        if c.is_numeric() {
            reference_numbers.push(c);
        } else {
            reference_characters.push(c);
        }
    }
    format!("{}{:0>4}", reference_characters, reference_numbers)
}

fn search_part<'a>(partlist: &'a [BomItem], footprint: &str, value: &str) -> Option<&'a BomItem> {
    partlist
        .iter()
        .find(|item| item.footprint == footprint && (item.value == value || item.value == "*"))
}

fn merge_item(item: &BomItem, part: Option<&BomItem>) -> BomItem {
    let datasheet = if let Some(part) = &part {
        part.datasheet.to_string()
    } else {
        item.datasheet.to_string()
    };
    let description = if let Some(part) = &part {
        part.description.to_string()
    } else {
        item.description.to_string()
    };
    let mouser_nr = if let Some(part) = &part {
        part.mouser_nr.to_string()
    } else {
        item.mouser_nr.to_string()
    };

    BomItem {
        amount: item.amount,
        references: item.references.clone(),
        value: item.value.to_string(),
        footprint: item.footprint.to_string(),
        datasheet,
        description,
        mouser_nr,
    }
}

/// Create the BOM for a Schema.
///
/// # Arguments
///
/// * `schema` - A Schema struct.
/// * `group`    - group equal items.
/// * `partlist` - A YAML file with the parts description.
/// * `return`   - Tuple with a Vec<BomItem> and the items not found in the partlist, when provided.
pub fn bom(
    schema: &Schema,
    group: bool,
    partlist: Option<PathBuf>,
) -> Result<(Vec<BomItem>, Option<Vec<BomItem>>), RecadError> {
    let partlist = if let Some(partlist) = partlist {
        Some(get_partlist(partlist.to_str().unwrap())?)
    } else {
        None
    };
    let mut bom_items: Vec<BomItem> = Vec::new();
    let mut missing_items: Vec<BomItem> = Vec::new();

    for item in &schema.items {
        if let SchemaItem::Symbol(symbol) = item {
            if symbol.unit == 1
                && symbol.on_board
                && symbol.in_bom
                && !symbol.lib_id.starts_with("power:")
                && !symbol.lib_id.starts_with("Mechanical:")
            {
                let bom_item = BomItem {
                    amount: 1,
                    references: vec![symbol.property(el::PROPERTY_REFERENCE).unwrap_or_default()],
                    value: symbol.property(el::PROPERTY_VALUE).unwrap_or_default(),
                    footprint: symbol.property("Footprint").unwrap_or_default(),
                    datasheet: symbol.property("Datasheet").unwrap_or_default(),
                    description: symbol.property("Description").unwrap_or_default(),
                    mouser_nr: String::new(),
                };

                if let Some(partlist) = &partlist {
                    let part = search_part(partlist, &bom_item.footprint, &bom_item.value);
                    if part.is_none() {
                        missing_items.push(bom_item.clone());
                    }
                    bom_items.push(merge_item(&bom_item, part));
                } else {
                    bom_items.push(bom_item);
                }
            }
        }
    }

    if group {
        let mut map: HashMap<String, Vec<&BomItem>> = HashMap::new();
        for item in &bom_items {
            let key = format!("{}:{}", item.value, item.footprint);
            map.entry(key).or_default().push(item);
        }
        bom_items = map
            .values()
            .map(|value| {
                let mut refs: Vec<String> = Vec::new();
                for v in value {
                    refs.push(v.references.first().unwrap().to_string());
                }
                BomItem {
                    amount: value.len(),
                    references: refs,
                    value: value[0].value.to_string(),
                    footprint: value[0].footprint.to_string(),
                    datasheet: value[0].datasheet.to_string(),
                    description: value[0].description.to_string(),
                    mouser_nr: value[0].mouser_nr.to_string(),
                }
            })
            .collect();
    }

    bom_items.sort_by(|a, b| {
        let ref_a = reference(&a.references[0]);
        let ref_b = reference(&b.references[0]);
        ref_a.partial_cmp(&ref_b).unwrap()
    });

    Ok((
        bom_items,
        if missing_items.is_empty() {
            None
        } else {
            Some(missing_items)
        },
    ))
}
