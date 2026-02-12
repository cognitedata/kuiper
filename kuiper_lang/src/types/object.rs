use std::{collections::BTreeMap, fmt::Display};

use serde::Serialize;
use serde_json::Value;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
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
            .or_else(|| {
                self.fields
                    .get(&ObjectField::Generic)
                    .cloned()
                    .map(|v| v.union_with(Type::null()))
            })
    }

    /// Produce a new object type that is the union of this object type and another.
    pub fn union_with(mut self, mut other: Object) -> Self {
        // We have to be conservative when merging objects. If either has a generic field,
        // any fields in the other object must be unioned with the generic field from the first.
        let left_union = self.fields.remove(&ObjectField::Generic);
        let right_union = other.fields.remove(&ObjectField::Generic);

        let mut res = BTreeMap::new();

        // First, make sure every field in the left object is unioned with the right generic field,
        // if it exists.
        for (key, value) in self.fields {
            if let Some(right_union) = &right_union {
                res.insert(key, value.union_with(right_union.clone()));
            } else {
                res.insert(key, value);
            }
        }

        // Next, merge the right object fields into the result, unioning each field with the left
        // generic field if it exists.
        for (key, mut value) in other.fields.into_iter() {
            match res.entry(key) {
                std::collections::btree_map::Entry::Vacant(e) => {
                    if let Some(left_union) = &left_union {
                        value = value.union_with(left_union.clone());
                    }
                    e.insert(value)
                }
                std::collections::btree_map::Entry::Occupied(mut e) => {
                    let mut val = e.get().clone();
                    val = val.union_with(value);
                    e.insert(val);
                    e.into_mut()
                }
            };
        }

        let union = match (left_union, right_union) {
            (Some(l), Some(r)) => Some(l.union_with(r)),
            (Some(v), None) | (None, Some(v)) => Some(v),
            (None, None) => None,
        };

        if let Some(union) = union {
            res.insert(ObjectField::Generic, union);
        }

        Object { fields: res }
    }

    /// Produce an object type from a constant JSON value.
    pub fn from_const(value: serde_json::Map<String, Value>) -> Self {
        let fields = value
            .into_iter()
            .map(|(k, v)| (ObjectField::Constant(k), Type::from_const(v)))
            .collect();
        Object { fields }
    }

    /// Return a union of all the elements in this object.
    pub fn element_union(&self) -> Type {
        let mut res = Type::never();
        for value in self.fields.values() {
            res = res.union_with(value.clone());
        }
        res
    }

    /// Add a field to this object, using a builder-like pattern.
    pub fn with_field(mut self, field: &str, ty: Type) -> Self {
        self.fields
            .insert(ObjectField::Constant(field.to_owned()), ty);
        self
    }

    /// Set the generic field of this object, using a builder-like pattern.
    pub fn with_generic_field(mut self, ty: Type) -> Self {
        self.fields.insert(ObjectField::Generic, ty);
        self
    }

    /// Push a field into this object type, unioning it with any existing field with the same name.
    pub fn push_field(&mut self, field: ObjectField, ty: Type) {
        match self.fields.entry(field) {
            std::collections::btree_map::Entry::Vacant(vacant) => {
                vacant.insert(ty);
            }
            std::collections::btree_map::Entry::Occupied(occupied) => {
                let (k, v) = occupied.remove_entry();
                let new_type = v.union_with(ty);
                self.fields.insert(k, new_type);
            }
        }
    }

    /// Check whether this object type accepts a field with the given key and type,
    /// i.e. whether a field with the given key and type can be assigned to this object type
    /// without changing it.
    pub fn accepts_field(&self, key: &str, ty: &Type) -> bool {
        // Either the specific field type accepts it, or the generic field type accepts it.
        if let Some(field_type) = self.fields.get(&ObjectField::Constant(key.to_string())) {
            ty.is_assignable_to(field_type)
        } else if let Some(field_type) = self.fields.get(&ObjectField::Generic) {
            ty.is_assignable_to(field_type)
        } else {
            false
        }
    }

    /// Check if this object type is assignable to another object type.
    /// This means that all fields in `self` must be accepted by `other`,
    /// and all fields in `other` must be either nullable or contained in `self`.
    pub fn is_assignable_to(&self, other: &Object) -> bool {
        // Check if each field in self is accepted by other.
        for (key, value) in &self.fields {
            match key {
                ObjectField::Constant(k) => {
                    if !other.accepts_field(k, value) {
                        return false;
                    }
                }
                ObjectField::Generic => {
                    if let Some(other_generic) = other.fields.get(&ObjectField::Generic) {
                        if !value.is_assignable_to(other_generic) {
                            return false;
                        }
                    } else {
                        // If other has no generic field, technically we can be received by
                        // _any_ field in other that is not explicitly matched:
                        let mut matched = false;
                        for field in other
                            .fields
                            .iter()
                            .filter(|(k, _)| !self.fields.contains_key(k))
                        {
                            if value.is_assignable_to(field.1) {
                                matched = true;
                                break;
                            }
                        }
                        if !matched {
                            return false;
                        }
                    }
                }
            }
        }

        // Check if each field in other is either nullable or contained in self.
        for (key, value) in &other.fields {
            // We don't care about generic fields here.
            let ObjectField::Constant(k) = key else {
                continue;
            };

            // If the field is nullable, it's fine if we don't have it.
            if Type::null().is_assignable_to(value) {
                continue;
            }
            // Otherwise, we must have the field in self, and it must be assignable.
            if let Some(self_value) = self.fields.get(&ObjectField::Constant(k.clone())) {
                if !self_value.is_assignable_to(value) {
                    return false;
                }
            } else if let Some(generic) = self.fields.get(&ObjectField::Generic) {
                if !generic.is_assignable_to(value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
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
        assert_eq!(
            obj.index_into("c"),
            Some(Type::Union(vec![Type::Float, Type::null()]))
        );
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

    #[test]
    fn test_object_union() {
        let mut fields1 = BTreeMap::new();
        fields1.insert(ObjectField::Constant("a".to_string()), Type::String);
        fields1.insert(
            ObjectField::Constant("b".to_string()),
            Type::from_const(123),
        );
        fields1.insert(ObjectField::Generic, Type::Float);
        let obj1 = Object { fields: fields1 };

        let mut fields2 = BTreeMap::new();
        fields2.insert(ObjectField::Constant("b".to_string()), Type::Boolean);
        fields2.insert(
            ObjectField::Constant("c".to_string()),
            Type::from_const("abc"),
        );
        let obj2 = Object { fields: fields2 };

        let un = obj1.union_with(obj2);
        let mut expected_fields = BTreeMap::new();
        expected_fields.insert(ObjectField::Constant("a".to_string()), Type::String);
        expected_fields.insert(
            ObjectField::Constant("b".to_string()),
            Type::Union(vec![Type::from_const(123), Type::Boolean]),
        );
        expected_fields.insert(
            ObjectField::Constant("c".to_string()),
            Type::Union(vec![Type::from_const("abc"), Type::Float]),
        );
        expected_fields.insert(ObjectField::Generic, Type::Float);
        let expected = Object {
            fields: expected_fields,
        };
        assert_eq!(un, expected);
    }
}
