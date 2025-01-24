use std::fmt::Display;

use regex::Regex;

#[derive(Debug)]
pub struct Macro {
    pub def: String,
    pub name: String,
}

impl Macro {
    pub fn from_expression(expr: &str) -> Result<Macro, &str> {
        let name_pattern = Regex::new(r"#(\w+)\s*:=").unwrap();
        let def_pattern = Regex::new(r":=\s*(.+)\s*;").unwrap();

        let name = name_pattern
            .captures(expr)
            .ok_or("Could not fetch name for macro")?
            .get(1)
            .ok_or("Could not fetch name for macro")?
            .as_str();

        let def = def_pattern
            .captures(expr)
            .ok_or("Could not fetch macro definition")?
            .get(1)
            .ok_or("Could not fetch macro definition")?
            .as_str();

        Ok(Macro {
            def: def.to_string(),
            name: name.to_string(),
        })
    }
}

impl Display for Macro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{} := {};", self.name, self.def)
    }
}
