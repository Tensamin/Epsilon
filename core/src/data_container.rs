use crate::data_types::DataTypes;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataKind {
    Bool,
    Number,
    Str,
    Array(Box<DataKind>),
    Container,
    Null,
}

#[derive(Debug, Clone, Eq)]
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
    pub fn container_from_map(map: &BTreeMap<DataTypes, DataValue>) -> DataValue {
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
    pub fn as_map(&self) -> Option<BTreeMap<DataTypes, DataValue>> {
        match self {
            DataValue::Container(c) => {
                let mut map = BTreeMap::new();
                for (key, value) in c {
                    map.insert(key.clone(), value.clone());
                }
                Some(map)
            }
            _ => None,
        }
    }
}

// ===========================================
// Implementation of Partial EQ & Hash
// ===========================================

impl PartialEq for DataValue {
    fn eq(&self, other: &Self) -> bool {
        use DataValue::*;

        match (self, other) {
            (BoolTrue, BoolTrue) | (BoolFalse, BoolFalse) => true,

            (BoolTrue, Bool(true)) | (Bool(true), BoolTrue) => true,

            (BoolFalse, Bool(false)) | (Bool(false), BoolFalse) => true,

            (Bool(a), Bool(b)) => a == b,

            (Number(a), Number(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Array(a), Array(b)) => a == b,
            (Container(a), Container(b)) => a == b,
            (Null, Null) => true,

            _ => false,
        }
    }
}

impl Hash for DataValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use DataValue::*;

        match self {
            BoolTrue | Bool(true) => {
                0u8.hash(state);
                true.hash(state);
            }
            BoolFalse | Bool(false) => {
                0u8.hash(state);
                false.hash(state);
            }
            Number(n) => {
                1u8.hash(state);
                n.hash(state);
            }
            Str(s) => {
                2u8.hash(state);
                s.hash(state);
            }
            Array(a) => {
                3u8.hash(state);
                a.hash(state);
            }
            Container(c) => {
                4u8.hash(state);
                c.hash(state);
            }
            Null => {
                5u8.hash(state);
            }
        }
    }
}

// ===========================================
// Tests
// ===========================================

#[cfg(test)]
#[allow(unused, dead_code)]
mod tests {
    use super::*;
    use crate::data_types::DataTypes;
    use std::collections::BTreeMap;

    fn sample_key_bool() -> DataTypes {
        DataTypes::accepted
    }

    fn sample_key_num() -> DataTypes {
        DataTypes::user_id
    }

    fn sample_key_str() -> DataTypes {
        DataTypes::username
    }

    fn sample_key_container() -> DataTypes {
        DataTypes::settings
    }

    fn sample_key_num_array() -> DataTypes {
        DataTypes::user_ids
    }

    #[test]
    fn test_kind_basic_types() {
        assert_eq!(DataValue::BoolTrue.kind(), DataKind::Bool);
        assert_eq!(DataValue::BoolFalse.kind(), DataKind::Bool);
        assert_eq!(DataValue::Bool(true).kind(), DataKind::Bool);
        assert_eq!(DataValue::Number(42).kind(), DataKind::Number);
        assert_eq!(DataValue::Str("hello".into()).kind(), DataKind::Str);
        assert_eq!(DataValue::Null.kind(), DataKind::Null);
    }

    #[test]
    fn test_kind_array() {
        let arr = DataValue::Array(vec![DataValue::Number(1), DataValue::Number(2)]);

        match arr.kind() {
            DataKind::Array(inner) => {
                assert_eq!(*inner, DataKind::Number);
            }
            _ => panic!("Expected Array kind"),
        }
    }

    #[test]
    #[should_panic]
    fn test_kind_array_empty_panics() {
        // This will panic due to unwrap() in kind()
        let arr = DataValue::Array(vec![]);
        arr.kind();
    }

    #[test]
    fn test_as_bool() {
        assert_eq!(DataValue::BoolTrue.as_bool(), Some(true));
        assert_eq!(DataValue::BoolFalse.as_bool(), Some(false));
        assert_eq!(DataValue::Bool(true).as_bool(), Some(true));
        assert_eq!(DataValue::Number(1).as_bool(), None);
    }

    #[test]
    fn test_as_str_and_string() {
        let value = DataValue::Str("hello".into());

        assert_eq!(value.as_str(), Some("hello"));
        assert_eq!(value.as_string(), Some("hello".to_string()));
        assert_eq!(DataValue::Number(10).as_str(), None);
    }

    #[test]
    fn test_as_number() {
        let value = DataValue::Number(123);
        assert_eq!(value.as_number(), Some(123));
        assert_eq!(DataValue::BoolTrue.as_number(), None);
    }

    #[test]
    fn test_as_array() {
        let arr = vec![DataValue::Number(1)];
        let value = DataValue::Array(arr.clone());

        assert_eq!(value.as_array(), Some(arr));
        assert_eq!(DataValue::Null.as_array(), None);
    }

    #[test]
    fn test_container_from_map_and_as_map() {
        let mut map = BTreeMap::new();
        map.insert(sample_key_num(), DataValue::Number(1));
        map.insert(sample_key_str(), DataValue::Str("x".into()));

        let container = DataValue::container_from_map(&map);

        let result_map = container.as_map().unwrap();

        assert_eq!(map.len(), result_map.len());
        assert_eq!(
            map.get(&sample_key_str()),
            result_map.get(&sample_key_str())
        );
        assert_eq!(
            map.get(&sample_key_num()),
            result_map.get(&sample_key_num())
        );
    }

    #[test]
    fn test_as_container() {
        let mut map = BTreeMap::new();
        map.insert(sample_key_bool(), DataValue::Number(10));

        let container = DataValue::container_from_map(&map);
        let vec_form = container.as_container().unwrap();

        assert_eq!(vec_form.len(), 1);
        assert_eq!(vec_form[0].1, DataValue::Number(10));
    }

    #[test]
    fn test_equality() {
        assert_eq!(DataValue::Number(5), DataValue::Number(5));
        assert_ne!(DataValue::Number(5), DataValue::Number(6));
        assert_eq!(DataValue::BoolTrue, DataValue::BoolTrue);
        assert_ne!(DataValue::BoolTrue, DataValue::BoolFalse);
        assert_eq!(DataValue::Bool(true), DataValue::BoolTrue);
        assert_ne!(DataValue::Bool(true), DataValue::BoolFalse);
    }
}
