//! Circuit struct to create the spice directives.

use types::{
    error::RecadError,
};

use indexmap::IndexMap;
use regex::Regex;
use std::sync::LazyLock;
use std::{
    fs::{self, File},
    io::Write,
};

pub static RE_SUBCKT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i:\.SUBCKT) ([a-zA-Z0-9]*) .*").unwrap());

pub static RE_MODEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i:\.model) ([a-zA-Z0-9]*) .*").unwrap());

pub static RE_INCLUDE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i:\.include) (.*)").unwrap());

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitItem {
    R(String, String, String, String),
    C(String, String, String, String),
    D(String, String, String, String),
    J(String, String, String, String, String),
    Q(String, String, String, String, String),
    X(String, Vec<String>, String),
    M(String, String, String, String, String, String),
    V(String, String, String, String),
}

///The Circuit struct represents a spice netlist.
#[derive(Debug, Clone, PartialEq)]
pub struct Circuit {
    name: String,
    pathlist: Vec<String>,
    items: Vec<CircuitItem>,
    subcircuits: IndexMap<String, (Vec<String>, Circuit)>,
    pub controls: Vec<String>,
    pub options: IndexMap<String, String>,
}

impl Circuit {
    pub fn new(name: String, pathlist: Vec<String>) -> Self {
        Self {
            name,
            pathlist,
            items: Vec::new(),
            subcircuits: IndexMap::new(),
            controls: Vec::new(),
            options: IndexMap::new(),
        }
    }

    //Add a resistor to the netlist.
    pub fn resistor(&mut self, reference: String, n0: String, n1: String, value: String) {
        self.items.push(CircuitItem::R(reference, n0, n1, value));
    }

    //Add a capacitor to the netlist.
    pub fn capacitor(&mut self, reference: String, n0: String, n1: String, value: String) {
        self.items.push(CircuitItem::C(reference, n0, n1, value));
    }

    //Add a diode to the netlist.
    pub fn diode(&mut self, reference: String, n0: String, n1: String, value: String) {
        self.items.push(CircuitItem::D(reference, n0, n1, value));
    }

    //Add a bjt transistor to the netlist.
    pub fn bjt(&mut self, reference: String, n0: String, n1: String, n2: String, value: String) {
        self.items
            .push(CircuitItem::Q(reference, n0, n1, n2, value));
    }

    //Add a bjt transistor to the netlist.
    pub fn jfet(&mut self, reference: String, n0: String, n1: String, n2: String, value: String) {
        self.items
            .push(CircuitItem::J(reference, n0, n1, n2, value));
    }

    // Add a MOSFET to the netlist.
    pub fn mosfet(
        &mut self,
        reference: String,
        nd: String,
        ng: String,
        ns: String,
        nb: String,
        value: String,
    ) {
        self.items
            .push(CircuitItem::M(reference, nd, ng, ns, nb, value));
    }

    pub fn circuit(
        &mut self,
        reference: String,
        n: Vec<String>,
        value: String,
    ) -> Result<(), RecadError> {
        self.items.push(CircuitItem::X(reference, n, value));
        Ok(())
    }

    pub fn subcircuit(
        &mut self,
        name: String,
        n: Vec<String>,
        circuit: Circuit,
    ) -> Result<(), RecadError> {
        self.subcircuits.insert(name, (n, circuit));
        Ok(())
    }

    pub fn voltage<S: AsRef<str>>(&mut self, reference: S, n1: S, n2: S, value: S) {
        self.items.push(CircuitItem::V(
            reference.as_ref().to_string(),
            n1.as_ref().to_string(),
            n2.as_ref().to_string(),
            value.as_ref().to_string(),
        ));
    }

    pub fn option(&mut self, option: String, value: String) {
        self.options.insert(option, value);
    }

    pub fn control(&mut self, control: String) {
        let mut lines: Vec<String> = control
            .lines()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .collect();
        self.controls.append(&mut lines);
    }

    pub fn save(&self, filename: Option<String>) -> Result<(), RecadError> {
        let mut out: Box<dyn Write> = if let Some(filename) = filename {
            Box::new(File::create(filename).unwrap())
        } else {
            Box::new(std::io::stdout())
        };

        for s in self.to_str(true).unwrap() {
            writeln!(out, "{}", s)?;
        }

        if !self.controls.is_empty() {
            writeln!(out, ".control")?;
            for c in &self.controls {
                writeln!(out, "{}", c)?;
            }
            writeln!(out, ".endc")?;
        }
        out.flush()?;
        Ok(())
    }

    pub fn set_value(&mut self, reference: &str, value: &str) -> Result<(), RecadError> {
        for item in &mut self.items.iter_mut() {
            match item {
                CircuitItem::R(r, _, _, ref mut v) => {
                    if reference == r {
                        *v = value.to_string();
                        return Ok(());
                    }
                }
                CircuitItem::C(r, _, _, ref mut v) => {
                    if reference == r {
                        *v = value.to_string();
                        return Ok(());
                    }
                }
                CircuitItem::D(r, _, _, ref mut v) => {
                    if reference == r {
                        *v = value.to_string();
                        return Ok(());
                    }
                }
                CircuitItem::J(_, _, _, _, _) => {}
                CircuitItem::Q(_, _, _, _, _) => {}
                CircuitItem::X(_, _, _) => {}
                CircuitItem::M(_, _, _, _, _, _) => {}
                CircuitItem::V(r, _, _, ref mut v) => {
                    if reference == r {
                        *v = value.to_string();
                        return Ok(());
                    }
                }
            }
        }
        Err(RecadError::Spice(format!(
            "Spice model not found {}",
            reference
        )))
    }
}

impl Circuit {
    /// Adds a generic component (like a Chip, subcircuit, or unknown symbol)
    /// This handles the "X" prefix or standard components defined by RefDes
    pub fn generic_component(&mut self, ref_des: String, nodes: Vec<String>, value: String, sim_device: String) {
        let prefix = if !sim_device.is_empty() {
            match sim_device.to_uppercase().as_str() {
                "SUBCKT" => "X".to_string(),
                "BJT" => "Q".to_string(),
                "MOSFET" => "M".to_string(),
                "JFET" => "J".to_string(),
                "DIODE" => "D".to_string(),
                // For "R", "C", "L", "V", "I", etc., the first char is correct
                p => p.chars().next().unwrap_or('X').to_uppercase().to_string(),
            }
        } else {
            ref_des.chars().next().unwrap_or('X').to_uppercase().to_string()
        };

        match prefix.as_str() {
            "R" if nodes.len() == 2 => {
                self.resistor(ref_des, nodes[0].clone(), nodes[1].clone(), value)
            }
            "C" if nodes.len() == 2 => {
                self.capacitor(ref_des, nodes[0].clone(), nodes[1].clone(), value)
            }
            "D" if nodes.len() == 2 => {
                self.diode(ref_des, nodes[0].clone(), nodes[1].clone(), value)
            }
            "Q" if nodes.len() == 3 => self.bjt(
                ref_des,
                nodes[0].clone(),
                nodes[1].clone(),
                nodes[2].clone(),
                value,
            ),
            "J" if nodes.len() == 3 => self.jfet(
                ref_des,
                nodes[0].clone(),
                nodes[1].clone(),
                nodes[2].clone(),
                value,
            ),
            "M" if nodes.len() == 4 => self.mosfet(
                ref_des,
                nodes[0].clone(),
                nodes[1].clone(),
                nodes[2].clone(),
                nodes[3].clone(),
                value,
            ),
            "V" if nodes.len() == 2 => {
                self.voltage(ref_des, nodes[0].clone(), nodes[1].clone(), value)
            }
            _ => {
                // Default to X (Subcircuit) or just treat as generic X element
                let _ = self.circuit(ref_des, nodes, value);
            }
        }
    }

    pub fn get_includes(&self, key: String) -> Result<IndexMap<String, String>, RecadError> {
        spdlog::debug!("load spice model: {key}");
        let mut result: IndexMap<String, String> = IndexMap::new();
        for path in &self.pathlist {
            let content = match fs::read_dir(path) {
                Ok(content) => content,
                Err(e) => {
                    return Err(RecadError::Spice(format!(
                        "directory not found {} ({})",
                        path, e
                    )))
                }
            };
            for entry in content {
                let dir = entry.unwrap();
                if dir.path().is_file() {
                    let content = fs::read_to_string(dir.path())?;
                    for cap in RE_SUBCKT.captures_iter(&content) {
                        let text1 = cap.get(1).map_or("", |m| m.as_str());
                        if text1 == key {
                            result.insert(key, dir.path().to_str().unwrap().to_string());
                            if let Some(caps) = RE_INCLUDE.captures(&content) {
                                for cap in caps.iter().skip(1) {
                                    let text1 = cap.map_or("", |m| m.as_str());
                                    if !text1.contains('/') {
                                        let parent = dir.path().parent().unwrap().join(text1);
                                        result.insert(text1.to_string(), parent.to_str().expect("get path").to_string());
                                    } else {
                                        result.insert(text1.to_string(), text1.to_string());
                                    }
                                }
                            }
                            return Ok(result);
                        }
                    }
                    for cap in RE_MODEL.captures_iter(&content) {
                        let text1 = cap.get(1).map_or("", |m| m.as_str());
                        if text1 == key {
                            result.insert(key, dir.path().to_str().unwrap().to_string());
                            if let Some(caps) = RE_INCLUDE.captures(&content) {
                                for cap in caps.iter().skip(1) {
                                    let text1 = cap.map_or("", |m| m.as_str());
                                    if !text1.contains('/') {
                                        //when there is no slash i could be
                                        //a relative path.
                                        let mut parent = dir
                                            .path()
                                            .parent()
                                            .unwrap()
                                            .to_str()
                                            .unwrap()
                                            .to_string();
                                        parent += "/";
                                        parent += text1;
                                        result.insert(text1.to_string(), parent.to_string());
                                    } else {
                                        result.insert(text1.to_string(), text1.to_string());
                                    }
                                }
                            }
                            return Ok(result);
                        }
                    }
                }
            }
        }
        Err(RecadError::Spice(format!("Spice model not found {}", key)))
    }

    fn includes(&self) -> Result<Vec<String>, RecadError> {
        let mut includes: IndexMap<String, String> = IndexMap::new();
        for item in &self.items {
            spdlog::debug!("get spice includes: {:?}", item);
            if let CircuitItem::X(_, _, value) = item {
                if !includes.contains_key(value) && !self.subcircuits.contains_key(value) {
                    let incs = self.get_includes(value.to_string())?;
                    for (key, value) in incs {
                        includes.entry(key).or_insert(value);
                    }
                }
            } else if let CircuitItem::J(_, _, _, _, value) = item {
                if !includes.contains_key(value) && !self.subcircuits.contains_key(value) {
                    let incs = self.get_includes(value.to_string())?;
                    for (key, value) in incs {
                        includes.entry(key).or_insert(value);
                    }
                }
            } else if let CircuitItem::Q(_, _, _, _, value) = item {
                if !includes.contains_key(value) && !self.subcircuits.contains_key(value) {
                    let incs = self.get_includes(value.to_string())?;
                    for (key, value) in incs {
                        includes.entry(key).or_insert(value);
                    }
                }
            } else if let CircuitItem::D(_, _, _, value) = item {
                if !includes.contains_key(value) && !self.subcircuits.contains_key(value) {
                    let incs = self.get_includes(value.to_string())?;
                    for (key, value) in incs {
                        includes.entry(key).or_insert(value);
                    }
                }
            } else if let CircuitItem::M(_, _, _, _, _, value) = item {
                if !includes.contains_key(value) && !self.subcircuits.contains_key(value) {
                    let incs = self.get_includes(value.to_string())?;
                    for (key, value) in incs {
                        includes.entry(key).or_insert(value);
                    }
                }
            }
        }
        let mut result = Vec::new();
        for (_, v) in includes {
            result.push(format!(".include {}\n", v).to_string());
        }
        Ok(result)
    }

    pub fn to_str(&self, close: bool) -> Result<Vec<String>, RecadError> {
        let mut res = Vec::new();

        res.push(String::from(".title auto generated netlist file."));

        res.append(&mut self.includes()?);
        for (key, value) in &self.subcircuits {
            let nodes = value.0.join(" ");
            res.push(format!(".subckt {} {}", key, nodes));
            res.append(&mut value.1.to_str(false).unwrap());
            res.push(".ends".to_string());
        }

        for (key, value) in &self.options {
            res.push(format!(".{} {}", key, value));
        }

        for item in &self.items {
            match item {
                CircuitItem::R(reference, n0, n1, value) => {
                    if reference.starts_with('R') {
                        res.push(format!("{} {} {} {}", reference, n0, n1, value));
                    } else {
                        res.push(format!("R{} {} {} {}", reference, n0, n1, value));
                    }
                }
                CircuitItem::C(reference, n0, n1, value) => {
                    if reference.starts_with('C') {
                        res.push(format!("{} {} {} {}", reference, n0, n1, value));
                    } else {
                        res.push(format!("C{} {} {} {}", reference, n0, n1, value));
                    }
                }
                CircuitItem::D(reference, n0, n1, value) => {
                    if reference.starts_with('D') {
                        res.push(format!("{} {} {} {}", reference, n0, n1, value));
                    } else {
                        res.push(format!("D{} {} {} {}", reference, n0, n1, value));
                    }
                }
                CircuitItem::Q(reference, n0, n1, n2, value) => {
                    if reference.starts_with('Q') {
                        res.push(format!("{} {} {} {} {}", reference, n0, n1, n2, value));
                    } else {
                        res.push(format!("Q{} {} {} {} {}", reference, n0, n1, n2, value));
                    }
                }
                CircuitItem::J(reference, n0, n1, n2, value) => {
                    if reference.starts_with('Q') {
                        res.push(format!("{} {} {} {} {}", reference, n0, n1, n2, value));
                    } else {
                        res.push(format!("J{} {} {} {} {}", reference, n0, n1, n2, value));
                    }
                }
                CircuitItem::M(reference, nd, ng, ns, nb, value) => {
                    if reference.starts_with('M') {
                        res.push(format!(
                            "{} {} {} {} {} {}",
                            reference, nd, ng, ns, nb, value
                        ));
                    } else {
                        res.push(format!(
                            "M{} {} {} {} {} {}",
                            reference, nd, ng, ns, nb, value
                        ));
                    }
                }
                CircuitItem::X(reference, n, value) => {
                    let mut nodes: String = String::new();
                    for _n in n {
                        nodes += _n;
                        nodes += " ";
                    }
                    res.push(format!("X{} {}{}", reference, nodes, value));
                }
                CircuitItem::V(reference, n0, n1, value) => {
                    res.push(format!("V{} {} {} {}", reference, n0, n1, value));
                }
            }
        }

        if close {
            res.push(String::from(".end"));
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::RE_SUBCKT;

    #[test]
    fn test_subckt_regext() {
        let cap = RE_SUBCKT.captures_iter(".SUBCKT CMOS4007 1 2 3 4 5 6 7 8 9 10 11 12 13 14");
        let mut res = String::from("not found");
        for c in cap {
            res = c.get(1).map_or("", |m| m.as_str()).to_string();
        }
        assert_eq!("CMOS4007", res)
    }
}
