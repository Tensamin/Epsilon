use crate::data_types::DataTypes;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataKind {
    Bool,
    Number,
    Str,
    Array(Box<DataKind>),
    Container,
    Null,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DataValue {
    BoolTrue,
    BoolFalse,
    Bool(bool),
    Number(i64),
    Str(String),
    Array(Vec<DataValue>),
    Container(Vec<(DataTypes, DataValue)>),
    Null,
}
impl DataValue {
    pub fn container_from_map(map: &HashMap<DataTypes, DataValue>) -> DataValue {
        let mut container = Vec::new();
        for (key, value) in map {
            container.push((key.clone(), value.clone()));
        }
        DataValue::Container(container)
    }
    pub fn kind(&self) -> DataKind {
        match self {
            DataValue::Bool(_) => DataKind::Bool,
            DataValue::BoolTrue => DataKind::Bool,
            DataValue::BoolFalse => DataKind::Bool,
            DataValue::Number(_) => DataKind::Number,
            DataValue::Str(_) => DataKind::Str,
            DataValue::Array(a) => DataKind::Array(Box::new(a.first().unwrap().kind())),
            DataValue::Container(_) => DataKind::Container,
            DataValue::Null => DataKind::Null,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DataValue::BoolTrue => Some(true),
            DataValue::BoolFalse => Some(false),
            DataValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DataValue::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_string(&self) -> Option<String> {
        match self {
            DataValue::Str(s) => Some(s.clone()),
            _ => None,
        }
    }
    pub fn as_number(&self) -> Option<i64> {
        match self {
            DataValue::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<Vec<DataValue>> {
        match self {
            DataValue::Array(a) => Some(a.clone()),
            _ => None,
        }
    }
    pub fn as_container(&self) -> Option<Vec<(DataTypes, DataValue)>> {
        match self {
            DataValue::Container(c) => Some(c.clone()),
            _ => None,
        }
    }
    pub fn as_map(&self) -> Option<HashMap<DataTypes, DataValue>> {
        match self {
            DataValue::Container(c) => {
                let mut map = HashMap::new();
                for (key, value) in c {
                    map.insert(key.clone(), value.clone());
                }
                Some(map)
            }
            _ => None,
        }
    }
}
