use std::collections::BTreeMap;

/// A map/tile property value (xTile property types).
#[derive(Clone, Debug, PartialEq)]
pub enum PropValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    Str(String),
}

impl PropValue {
    pub fn as_string(&self) -> String {
        match self {
            PropValue::Bool(b) => if *b { "T".into() } else { "F".into() },
            PropValue::Int(i) => i.to_string(),
            PropValue::Float(f) => f.to_string(),
            PropValue::Str(s) => s.clone(),
        }
    }

    /// Interpret as a truthy flag the way map data uses them ("T", "true", true).
    pub fn truthy(&self) -> bool {
        match self {
            PropValue::Bool(b) => *b,
            PropValue::Int(i) => *i != 0,
            PropValue::Float(f) => *f != 0.0,
            PropValue::Str(s) => {
                let s = s.trim();
                s.eq_ignore_ascii_case("t") || s.eq_ignore_ascii_case("true") || s == "1" || s.eq_ignore_ascii_case("y")
            }
        }
    }
}

pub type Props = BTreeMap<String, PropValue>;

pub trait PropsExt {
    fn str(&self, key: &str) -> Option<String>;
    fn flag(&self, key: &str) -> bool;
}

impl PropsExt for Props {
    fn str(&self, key: &str) -> Option<String> {
        self.get(key).map(|v| v.as_string())
    }
    fn flag(&self, key: &str) -> bool {
        self.get(key).map(|v| v.truthy()).unwrap_or(false)
    }
}
