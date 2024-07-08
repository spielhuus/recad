use crate::{
    gr::{self, Arc, Circle, Color, GraphicItem, Line, PaperSize, Polyline, Property, Rectangle},
    schema::{
        Bus, BusEntry, ConnectionType, GlobalLabel, HierarchicalLabel, HierarchicalPin, HierarchicalSheet, Instance, Junction, LocalLabel, NetclassFlag, NoConnect, ProjectInstance, SchemaItem, Symbol, Text, TextBox, Wire
    },
    sexp::{constants::el, Sexp, SexpQuery, SexpString, SexpStringList, SexpTree, SexpValue},
    symbols::{ElectricalTypes, LibrarySymbol, Pin, PinGraphicalStyle, PinProperty},
    Error, Schema,
};

macro_rules! error_if_none {
    ($value:expr, $msg:expr) => {
        match $value {
            None => Err(Error(el::SEXP.to_string(), $msg.to_string())),
            Some(x) => Ok(x),
        }
    };
}

impl std::convert::From<SexpTree> for Result<Schema, Error> {
    fn from(sexp: SexpTree) -> Self {
        let mut schema = Schema::default();
        for node in sexp.root().unwrap().nodes() {
            match node.name.as_str() {
                el::UUID => schema.uuid = node.get(0).unwrap(),
                el::GENERATOR => schema.generator = node.get(0).unwrap(),
                "generator_version" => schema.generator_version = node.get(0),
                "version" => schema.version = node.get(0).unwrap(),
                el::JUNCTION => schema
                    .items
                    .push(SchemaItem::Junction(Into::<Result<Junction, Error>>::into(node)?)),
                el::PAPER => schema.paper = PaperSize::from(&SexpString::get(node, 0).unwrap()),
                el::WIRE => {
                    schema.items.push(SchemaItem::Wire(Into::<Result<Wire, Error>>::into(node)?));
                }
                el::BUS => schema.items.push(SchemaItem::Bus(Into::<Result<Bus, Error>>::into(node)?)),
                el::BUS_ENTRY => schema
                    .items
                    .push(SchemaItem::BusEntry(Into::<Result<BusEntry, Error>>::into(node)?)),
                el::LABEL => schema
                    .items
                    .push(SchemaItem::LocalLabel(Into::<Result<LocalLabel, Error>>::into(node)?)),
                el::GLOBAL_LABEL => schema
                    .items
                    .push(SchemaItem::GlobalLabel(Into::<Result<GlobalLabel, Error>>::into(node)?)),
                el::NO_CONNECT => schema
                    .items
                    .push(SchemaItem::NoConnect(Into::<Result<NoConnect, Error>>::into(node)?)),
                el::TITLE_BLOCK => schema.title_block = node.into(),
                el::LIB_SYMBOLS => {
                    schema.library_symbols = node
                        .query(el::SYMBOL)
                        .map(|s| Into::<Result<LibrarySymbol, Error>>::into(s).unwrap())
                        .collect()
                }
                el::SYMBOL => schema.items.push(SchemaItem::Symbol(node.into())),
                el::CIRCLE => schema.items.push(SchemaItem::Circle(Into::<
                    Result<Circle, Error>,
                >::into(
                    node
                )?)),
                el::POLYLINE => {
                    schema.items.push(SchemaItem::Polyline(
                        Into::<Result<Polyline, Error>>::into(node)?,
                    ))
                }
                el::RECTANGLE => {
                    schema.items.push(SchemaItem::Rectangle(
                        Into::<Result<Rectangle, Error>>::into(node)?,
                    ))
                }
                el::ARC => schema
                    .items
                    .push(SchemaItem::Arc(Into::<Result<Arc, Error>>::into(node)?)),
                el::TEXT => schema
                    .items
                    .push(SchemaItem::Text(Into::<Result<Text, Error>>::into(node)?)),
                el::TEXT_BOX => schema
                    .items
                    .push(SchemaItem::TextBox(Into::<Result<TextBox, Error>>::into(node)?)),
                el::SHEET => schema
                    .items
                    .push(SchemaItem::HierarchicalSheet(Into::<Result<HierarchicalSheet, Error>>::into(node)?)),
                el::HIERARCHICAL_LABEL => schema
                    .items
                    .push(SchemaItem::HierarchicalLabel(Into::<Result<HierarchicalLabel, Error>>::into(node)?)),
                el::NETCLASS_FLAG => schema
                    .items
                    .push(SchemaItem::NetclassFlag(Into::<Result<NetclassFlag, Error>>::into(node)?)),
                el::SHEET_INSTANCES => {
                    let path = node.query(el::PATH).next().unwrap();
                    schema.sheet_instances = vec![Instance {
                        project: String::new(),
                        path: path.get(0).expect("mandatory field"),
                        reference: path.first("page").expect("mandatory field"),
                        unit: 0,
                    }]
                }
                _ => log::error!("unknown root node: {:?}", node.name),
            }
        }
        Ok(schema)
    }
}

impl std::convert::From<&Sexp> for Result<HierarchicalPin, Error> {
    fn from(sexp: &Sexp) -> Result<HierarchicalPin, Error> {
        Ok(HierarchicalPin {
            name: SexpString::get(sexp, 0).unwrap(),
            connection_type: ConnectionType::from(SexpString::get(sexp, 1).unwrap()),
            pos: sexp.into(),
            effects: sexp.into(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<HierarchicalSheet, Error> {
    fn from(sexp: &Sexp) -> Result<HierarchicalSheet, Error> {
        let size = sexp.query(el::SIZE).next().unwrap(); //TODO create error
        let instance = sexp.query(el::INSTANCES).next().unwrap();
        let project = instance.query(el::PROJECT).next().unwrap();
        let path = project.query(el::PATH).next().unwrap();
        Ok(HierarchicalSheet {
            pos: sexp.into(),
            width: size.get(0).unwrap(),
            height: size.get(1).unwrap(),
            fields_autoplaced: SexpString::first(sexp, el::FIELDS_AUTOPLACED)
                .unwrap_or(el::YES.to_string())
                == el::YES,
            stroke: sexp.into(),
            fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
            props: properties(sexp),
            pins: sexp.query(el::PIN).map(|p| { 
                Into::<Result<HierarchicalPin, Error>>::into(p).unwrap()
            }).collect(),
            instances: vec![ProjectInstance { 
                project_name: project.get(0).unwrap(),
                path: path.get(0).unwrap(),
                page_number: path.first(el::PAGE).unwrap(),
            }],
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<HierarchicalLabel, Error> {
    fn from(sexp: &Sexp) -> Result<HierarchicalLabel, Error> {
        Ok(HierarchicalLabel {
            text: SexpString::get(sexp, 0).unwrap(),
            shape: sexp.first(el::SHAPE),
            pos: sexp.into(),
            fields_autoplaced: SexpString::first(sexp, el::FIELDS_AUTOPLACED)
                .unwrap_or(el::YES.to_string())
                == el::YES,
            props: properties(sexp),
            effects: sexp.into(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<NetclassFlag, Error> {
    fn from(sexp: &Sexp) -> Result<NetclassFlag, Error> {
        Ok(NetclassFlag {
            name: SexpString::get(sexp, 0).unwrap(),
            length: sexp.first(el::LENGTH).unwrap(),
            shape: sexp.first(el::SHAPE),
            pos: sexp.into(),
            fields_autoplaced: SexpString::first(sexp, el::FIELDS_AUTOPLACED)
                .unwrap_or(el::YES.to_string())
                == el::YES,
            effects: sexp.into(),
            props: properties(sexp),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<Wire, Error> {
    fn from(sexp: &Sexp) -> Result<Wire, Error> {
        Ok(Wire {
            pts: sexp.into(),
            stroke: sexp.into(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<Bus, Error> {
    fn from(sexp: &Sexp) -> Result<Bus, Error> {
        Ok(Bus {
            pts: sexp.into(),
            stroke: sexp.into(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<BusEntry, Error> {
    fn from(sexp: &Sexp) -> Result<BusEntry, Error> {
        Ok(BusEntry {
            pos: sexp.into(),
            size: (
                //TODO error handling
                sexp.query(el::SIZE).next().unwrap().get(0).unwrap(),
                sexp.query(el::SIZE).next().unwrap().get(1).unwrap(),
            ),
            stroke: sexp.into(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<LocalLabel, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(LocalLabel {
            text: error_if_none!(sexp.get(0), "text is mandatory for label.")?,
            pos: sexp.into(),
            effects: sexp.into(),
            color: Into::<Result<Color, Error>>::into(sexp).ok(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
            fields_autoplaced: SexpString::first(sexp, el::FIELDS_AUTOPLACED)
                .unwrap_or(el::YES.to_string())
                == el::YES,
        })
    }
}

impl std::convert::From<&Sexp> for Result<GlobalLabel, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(GlobalLabel {
            text: error_if_none!(sexp.get(0), "text is mandatory for label.")?,
            shape: sexp.first(el::SHAPE),
            pos: sexp.into(),
            fields_autoplaced: SexpString::first(sexp, el::FIELDS_AUTOPLACED)
                .unwrap_or(el::YES.to_string())
                == el::YES,
            effects: sexp.into(),
            props: properties(sexp),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<Junction, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(Junction {
            pos: sexp.into(),
            diameter: sexp.first(el::DIAMETER).unwrap_or(0.0),
            color: Into::<Result<Color, Error>>::into(sexp).ok(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<NoConnect, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(NoConnect {
            pos: sexp.into(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

impl std::convert::From<&Sexp> for Result<gr::FillType, Error> {
    fn from(sexp: &Sexp) -> Self {
        if let Some(fill) = sexp.query(el::FILL).next() {
            if let Some(filltype) = SexpString::first(fill, el::TYPE) {
                if filltype == el::COLOR {
                    Ok(gr::FillType::Color(Into::<Result<Color, Error>>::into(fill).unwrap()))
                } else {
                    Ok(gr::FillType::from(filltype.as_str()))
                }
            } else {
                Ok(gr::FillType::None)
            }
        } else {
            Ok(gr::FillType::None)
        }
    }
}

impl std::convert::From<&Sexp> for Result<Text, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(Text {
            text: error_if_none!(sexp.get(0), "text is mandatory for label.")?,
            pos: sexp.into(),
            effects: sexp.into(),
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
            exclude_from_sim: if let Some(exclude) = SexpString::first(sexp, el::EXCLUDE_FROM_SIM) {
                exclude == el::YES
            } else {
                false
            },
        })
    }
}

impl std::convert::From<&Sexp> for Result<TextBox, Error> {
    fn from(sexp: &Sexp) -> Self {
        let size = sexp.query(el::SIZE).next().unwrap(); //TODO create error
        Ok(TextBox {
            pos: sexp.into(),
            text: error_if_none!(sexp.get(0), "text is mandatory for text box.")?,
            width: size.get(0).unwrap(),
            height: size.get(1).unwrap(),
            stroke: sexp.into(),
            fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
            effects: sexp.into(),
            exclude_from_sim: if let Some(exclude) = SexpString::first(sexp, el::EXCLUDE_FROM_SIM) {
                exclude == el::YES
            } else {
                false
            },
            uuid: error_if_none!(sexp.first(el::UUID), "uuid is mandatory")?,
        })
    }
}

fn properties(node: &Sexp) -> Vec<Property> {
    node.query(el::PROPERTY)
        .collect::<Vec<&Sexp>>()
        .iter()
        .map(|x| Property {
            pos: (*x).into(),
            key: x.get(0).unwrap(),
            value: x.get(1).unwrap(),
            effects: (*x).into(),
        })
        .collect()
}

impl std::convert::From<&Sexp> for Result<Circle, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(gr::Circle {
            center: sexp.query(el::CENTER).next().unwrap().into(),
            radius: sexp.first(el::RADIUS).unwrap(),
            stroke: sexp.into(),
            fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
            uuid: sexp.first(el::UUID),
        })
    }
}

impl std::convert::From<&Sexp> for Result<Line, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(Line {
            pts: sexp.into(),
            stroke: sexp.into(),
            fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
            uuid: sexp.first(el::UUID),
        })
    }
}

impl std::convert::From<&Sexp> for Result<Polyline, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(Polyline {
            pts: sexp.into(),
            stroke: sexp.into(),
            fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
            uuid: sexp.first(el::UUID),
        })
    }
}

impl std::convert::From<&Sexp> for Result<Rectangle, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(Rectangle {
            start: sexp.query(el::START).next().unwrap().into(),
            end: sexp.query(el::END).next().unwrap().into(),
            stroke: sexp.into(),
            fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
            uuid: sexp.first(el::UUID),
        })
    }
}

impl std::convert::From<&Sexp> for Result<Arc, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(Arc {
            start: sexp.query(el::START).next().unwrap().into(),
            mid: sexp.query(el::MID).next().unwrap().into(),
            end: sexp.query(el::END).next().unwrap().into(),
            stroke: sexp.into(),
            fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
            uuid: sexp.first(el::UUID),
        })
    }
}

fn pin_numbers(node: &Sexp) -> bool {
    let pin_numbers = node.query(el::PIN_NUMBERS).collect::<Vec<&Sexp>>();
    if pin_numbers.is_empty() {
        true
    } else {
        !<Sexp as SexpQuery<Vec<String>>>::values(
            pin_numbers.first().expect("tested before for is_empty"),
        )
        .contains(&el::HIDE.to_string())
    }
}

fn pin_names(node: &Sexp) -> bool {
    let pin_names = node.query(el::PIN_NAMES).collect::<Vec<&Sexp>>();
    if pin_names.is_empty() {
        true
    } else {
        !<Sexp as SexpQuery<Vec<String>>>::values(pin_names.first().expect("tested before"))
            .contains(&el::HIDE.to_string())
    }
}

pub fn pin_names_offset(sexp: &Sexp) -> Option<f32> {
    if let Some(names) = sexp.query(el::PIN_NAMES).next() {
        if let Some(offset) = <Sexp as SexpValue<f32>>::first(names, el::OFFSET) {
            return Some(offset);
        }
    }
    None
}

impl std::convert::From<&Sexp> for Result<LibrarySymbol, Error> {
    fn from(sexp: &Sexp) -> Self {
        Ok(LibrarySymbol {
            lib_id: error_if_none!(sexp.get(0), "lib_id is mandatory on library symbol")?,
            extends: sexp.first(el::EXTENDS),
            power: sexp.query(el::POWER).next().is_some(),
            exclude_from_sim: if let Some(exclude) = SexpString::first(sexp, el::EXCLUDE_FROM_SIM) {
                exclude == el::YES
            } else {
                false
            },
            in_bom: SexpString::first(sexp, el::IN_BOM).unwrap_or(el::YES.to_string()) == el::YES,
            on_board: SexpString::first(sexp, el::ON_BOARD).unwrap_or(el::YES.to_string())
                == el::YES,
            props: properties(sexp),
            graphics: sexp
                .nodes()
                .filter_map(|node| match node.name.as_str() {
                    el::ARC => Some(GraphicItem::Arc(
                        Into::<Result<Arc, Error>>::into(node).unwrap(),
                    )),
                    el::CIRCLE => Some(GraphicItem::Circle(
                        Into::<Result<Circle, Error>>::into(node).unwrap(),
                    )),
                    el::CURVE => Some(gr::GraphicItem::Curve(gr::Curve {
                        pts: node.into(),
                        stroke: node.into(),
                        fill: Into::<Result<gr::FillType, Error>>::into(sexp).unwrap(),
                    })),
                    el::POLYLINE => Some(GraphicItem::Polyline(
                        Into::<Result<Polyline, Error>>::into(node).unwrap(),
                    )),
                    el::LINE => Some(GraphicItem::Line(
                        Into::<Result<Line, Error>>::into(node).unwrap(),
                    )),
                    el::RECTANGLE => Some(GraphicItem::Rectangle(
                        Into::<Result<Rectangle, Error>>::into(node).unwrap(),
                    )),
                    el::TEXT => Some(gr::GraphicItem::Text(gr::Text {
                        text: node.get(0).expect("text is required"),
                        pos: node.into(),
                        effects: node.into(),
                        uuid: None,
                    })),
                    _ => {
                        if node.name != el::PIN
                            && node.name != el::SYMBOL
                            && node.name != el::POWER
                            && node.name != el::PIN_NUMBERS
                            && node.name != el::PIN_NAMES
                            && node.name != el::IN_BOM
                            && node.name != el::ON_BOARD
                            && node.name != el::EXCLUDE_FROM_SIM
                            && node.name != el::PROPERTY
                            && node.name != el::EXTENDS
                        {
                            panic!("unknown graphic type: {}", node.name);
                        }
                        None
                    }
                })
                .collect(),
            pins: sexp
                .nodes()
                .filter_map(|node| match node.name.as_str() {
                    el::PIN => Some(Pin {
                        electrical_type: ElectricalTypes::from(
                            SexpString::get(node, 0).unwrap().as_str(),
                        ),
                        graphical_style: PinGraphicalStyle::from(
                            SexpString::get(node, 1).unwrap().as_str(),
                        ),
                        pos: node.into(),
                        length: <Sexp as SexpValue<f32>>::first(node, el::LENGTH)
                            .expect("required"),
                        hide: SexpStringList::values(node).contains(&el::HIDE.to_string()),
                        name: {
                            let name = node.query(el::NAME).next().unwrap();
                            PinProperty {
                                name: name.get(0).unwrap(),
                                effects: name.into(),
                            }
                        },
                        number: {
                            let number = node.query(el::NUMBER).next().unwrap();
                            PinProperty {
                                name: number.get(0).unwrap(),
                                effects: number.into(),
                            }
                        },
                    }),
                    _ => None,
                })
                .collect(),
            pin_numbers: pin_numbers(sexp),
            pin_names: pin_names(sexp),
            pin_names_offset: pin_names_offset(sexp),
            units: sexp
                .query(el::SYMBOL)
                .map(|s| Into::<Result<LibrarySymbol, Error>>::into(s).unwrap())
                .collect::<Vec<LibrarySymbol>>(),
            unit_name: sexp.first("unit_name"), //TODO check name in sexp file.
        })
    }
}

impl std::convert::From<&Sexp> for Symbol {
    fn from(sexp: &Sexp) -> Self {
        Symbol {
            lib_id: sexp.first(el::LIB_ID).unwrap(),
            pos: sexp.into(),
            unit: sexp.first(el::SYMBOL_UNIT).unwrap(),
            mirror: sexp.first(el::MIRROR),
            in_bom: SexpString::first(sexp, el::IN_BOM).expect("required field") == el::YES,
            on_board: SexpString::first(sexp, el::ON_BOARD).unwrap() == el::YES,
            exclude_from_sim: if let Some(exclude) = SexpString::first(sexp, el::EXCLUDE_FROM_SIM) {
                exclude == el::YES
            } else {
                false
            },
            dnp: if let Some(dnp) = SexpString::first(sexp, el::DNP) {
                dnp == el::YES
            } else {
                false
            },
            uuid: sexp.first(el::UUID).unwrap(),
            props: properties(sexp),
            pins: sexp
                .query(el::PIN)
                .map(|p| (p.get(0).unwrap(), p.first(el::UUID).unwrap()))
                .collect(),
            instances: {
                let instances = sexp.query(el::INSTANCES).next().expect("mandatory field");
                let project = instances.query(el::PROJECT).next().unwrap();
                let path = project.query(el::PATH).next().unwrap();
                vec![Instance {
                    project: project.get(0).expect("mandatory field"),
                    path: path.get(0).expect("mandatory field"),
                    reference: path.first(el::REFERENCE).expect("mandatory field"),
                    unit: path.first(el::SYMBOL_UNIT).expect("mandatory field"),
                }]
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sexp::parser::SexpParser;
    use crate::{
        gr::{Pt, Pts, Stroke, StrokeType, TitleBlock},
        schema::Wire,
        sexp::SexpTree,
        Error,
    };

    #[test]
    fn empty_schema() {
        let schema = r#"
            (kicad_sch (version 20231120) (generator "eeschema") (generator_version "8.0")
              (paper "A4")
              (lib_symbols)
              (symbol_instances)
            )"#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let Ok(schema) = tree.into() else {
            panic!();
        };

        assert_eq!("20231120", schema.version);
        assert_eq!("eeschema", schema.generator);
        assert_eq!("8.0", schema.generator_version.unwrap());
        assert_eq!("A4", schema.paper.to_string());
    }

    #[test]
    fn title_block() {
        let schema = r#"
          (title_block
            (title "summe")
            (date "2021-05-30")
            (rev "R02")
            (company "company")
            (comment 1 "schema for pcb")
            (comment 2 "DC coupled mixer")
            (comment 3 "comment 3")
            (comment 4 "License CC BY 4.0 - Attribution 4.0 International")
            (comment 5 "comment 5")
            (comment 6 "comment 6")
            (comment 7 "comment 7")
            (comment 8 "comment 8")
            (comment 9 "comment 9")
          )"#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let tb: TitleBlock = tree.root().unwrap().into();
        assert_eq!("summe".to_string(), tb.title.unwrap());
        assert_eq!("2021-05-30".to_string(), tb.date.unwrap());
        assert_eq!("R02".to_string(), tb.revision.unwrap());
        assert_eq!("company".to_string(), tb.company_name.unwrap());
        assert_eq!(9, tb.comment.len());
        assert_eq!(
            (1, "schema for pcb".to_string()),
            *tb.comment.first().unwrap()
        );
    }

    #[test]
    fn into_stroke() {
        let schema = r#"
            (something
                (stroke
                        (width 0.1)
                        (type dash)
                )
            )
        "#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let stroke: Stroke = tree.root().unwrap().into();

        assert_eq!(0.1, stroke.width);
        assert_eq!(StrokeType::Dash, stroke.stroke_type.unwrap());
    }

    #[test]
    fn wire_schema() {
        let schema = r#"
                (wire
                        (pts
                                (xy 163.83 107.95) (xy 163.83 110.49)
                        )
                        (stroke
                                (width 0)
                                (type default)
                        )
                        (uuid "7a8a9d59-7b8a-4c1b-ab50-9119def7e130")
                )
        )"#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let Ok(wire) = Into::<Result<Wire, Error>>::into(tree.root().unwrap()) else {
            panic!();
        };
        assert_eq!(
            Pts(vec![
                Pt {
                    x: 163.83,
                    y: 107.95
                },
                Pt {
                    x: 163.83,
                    y: 110.49
                },
            ]),
            wire.pts
        );

        assert_eq!(0.0, wire.stroke.width);
        assert_eq!(Some(StrokeType::Default), wire.stroke.stroke_type);
    }
}
