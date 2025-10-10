use std::{collections::BTreeMap, fmt::Display};

use serde::Serialize;

use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A field in a JSON object, either a constant field name, or a generic field.
pub enum ObjectField {
    /// A constant, known field name.
    Constant(String),
    /// Any other fields. This represents _all_ unknown fields in an object.
    Generic,
}

impl Serialize for ObjectField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ObjectField::Constant(name) => serializer.serialize_str(name),
            ObjectField::Generic => serializer.serialize_str("..."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A JSON object type, containing both specific, known fields, and a generic field type.
/// The generic field type is used for fields that are not explicitly listed in the object,
/// making it possible to represent both "maps", and "structures".
pub struct Object {
    /// The fields in the object, mapping from field name (or generic) to type.
    pub fields: BTreeMap<ObjectField, Type>,
}

impl Serialize for Object {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.fields.serialize(serializer)
    }
}

impl Object {
    /// Index into the object type, returning the type of the field if it could exist.
    /// This only returns `None` if the field _cannot_ exist. This means that even if it does
    /// return `Some`, `null` might be a valid option.
    pub fn index_into(&self, key: &str) -> Option<Type> {
        self.fields
            .get(&ObjectField::Constant(key.to_string()))
            .cloned()
            // TODO: Union here once we have methods for creating unions.
            .or_else(|| self.fields.get(&ObjectField::Generic).cloned())
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut needs_comma = false;
        for (key, value) in &self.fields {
            if needs_comma {
                write!(f, ", ")?;
            } else {
                needs_comma = true;
            }
            match key {
                ObjectField::Constant(k) => write!(f, "{k}: ")?,
                ObjectField::Generic => write!(f, "...: ")?,
            }

            write!(f, "{}", value)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_indexing() {
        let mut fields = BTreeMap::new();
        fields.insert(ObjectField::Constant("a".to_string()), Type::String);
        fields.insert(
            ObjectField::Constant("b".to_string()),
            Type::from_const(123),
        );
        fields.insert(ObjectField::Generic, Type::Float);
        let obj = Object { fields };

        assert_eq!(obj.index_into("a"), Some(Type::String));
        assert_eq!(obj.index_into("b"), Some(Type::from_const(123)));
        assert_eq!(obj.index_into("c"), Some(Type::Float));
    }

    #[test]
    fn test_object_indexing_no_generic() {
        let mut fields = BTreeMap::new();
        fields.insert(ObjectField::Constant("a".to_string()), Type::String);
        fields.insert(
            ObjectField::Constant("b".to_string()),
            Type::from_const(123),
        );
        let obj = Object { fields };

        assert_eq!(obj.index_into("a"), Some(Type::String));
        assert_eq!(obj.index_into("b"), Some(Type::from_const(123)));
        assert_eq!(obj.index_into("c"), None);
    }
}
