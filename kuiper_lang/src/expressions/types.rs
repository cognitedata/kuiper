use std::{collections::HashMap, fmt::Display};

use logos::Span;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObjectField {
    Constant(String),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Type {
    Constant(Value),
    Object(Object),
    Sequence(Sequence),
    String,
    Integer,
    Float,
    Boolean,
    Union(Vec<Type>),
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub fields: HashMap<ObjectField, Type>,
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
    pub fn union_with(self, other: Object) -> Self {
        let mut res = self.fields.clone();
        for (key, value) in other.fields {
            res.entry(key)
                .and_modify(|e| *e = e.clone().union_with(value.clone()))
                .or_insert(value);
        }
        Object { fields: res }
    }

    pub fn index_into(&self, key: &str) -> Option<Type> {
        self.fields
            .get(&ObjectField::Constant(key.to_string()))
            .cloned()
            .or_else(|| self.fields.get(&ObjectField::Generic).cloned())
    }

    pub fn element_union(&self) -> Type {
        let mut ty = Type::Union(Vec::new());
        for elem in self.fields.values() {
            ty = ty.union_with(elem.clone());
        }
        ty
    }

    pub fn accepts_field(&self, key: &str, typ: &Type) -> bool {
        if let Some(field_type) = self.fields.get(&ObjectField::Constant(key.to_string())) {
            typ.is_assignable_to(field_type)
        } else if let Some(field_type) = self.fields.get(&ObjectField::Generic) {
            typ.is_assignable_to(field_type)
        } else {
            false
        }
    }

    pub fn accepts_field_key(&self, key: &ObjectField, typ: &Type) -> bool {
        if let Some(field_type) = self.fields.get(key) {
            typ.is_assignable_to(field_type)
        } else if let Some(field_type) = self.fields.get(&ObjectField::Generic) {
            typ.is_assignable_to(field_type)
        } else {
            false
        }
    }

    pub fn can_have_field(&self, key: &str, typ: &Type) -> bool {
        if let Some(field_type) = self.fields.get(&ObjectField::Constant(key.to_string())) {
            field_type.is_assignable_to(typ)
        } else if let Some(field_type) = self.fields.get(&ObjectField::Generic) {
            field_type.is_assignable_to(typ)
        } else {
            false
        }
    }

    pub fn is_assignable_to(&self, other: &Object) -> bool {
        // Each element in self must be accepted by other.
        for (self_key, self_type) in &self.fields {
            if !other.accepts_field_key(self_key, self_type) {
                return false;
            }
        }
        // Each non-generic element in other must be present in self, or be nullable.
        for (other_key, other_type) in &other.fields {
            if Type::null().is_assignable_to(other_type) {
                // If the other type is null, we can skip it.
                continue;
            }

            if let ObjectField::Constant(_) = other_key {
                if let Some(field_type) = self.fields.get(other_key) {
                    // If the field exists in self, it must be assignable to the other type.
                    if !field_type.is_assignable_to(other_type) {
                        return false;
                    }
                } else if let Some(generic_type) = self.fields.get(&ObjectField::Generic) {
                    // If the field does not exist in self, but there is a generic field,
                    // it must be assignable to the other type.
                    if !generic_type.is_assignable_to(other_type) {
                        return false;
                    }
                } else {
                    // If there is no generic field, we cannot assign this field.
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    pub elements: Vec<Type>,
    pub end_dynamic: Option<Box<Type>>,
}

impl Serialize for Sequence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.elements.serialize(serializer)
    }
}

impl Sequence {
    pub fn union_with(self, other: Sequence) -> Self {
        let mut res = Vec::new();
        let mut iter_1 = self.elements.into_iter();
        let mut iter_2 = other.elements.into_iter();

        loop {
            let Some(elem_1) = iter_1.next() else {
                break;
            };
            let Some(elem_2) = iter_2.next() else {
                break;
            };
            res.push(elem_1.union_with(elem_2));
        }

        let mut end_dynamic = None;
        if let Some(end_dynamic_1) = self.end_dynamic {
            if let Some(end_dynamic_2) = other.end_dynamic {
                // Both sequences have dynamic end, we need to union with anything
                // left in the iterators.
                let mut dynamic = end_dynamic_1.union_with(end_dynamic_2.as_ref().clone());
                for elem in iter_1.chain(iter_2) {
                    dynamic = dynamic.union_with(elem);
                }
                end_dynamic = Some(Box::new(dynamic));
            } else {
                // Only sequence 1 has dynamic end, so we can include anything in it
                res.extend(iter_1);
            }
        } else if let Some(end_dynamic_2) = other.end_dynamic {
            res.push(*end_dynamic_2);
            res.extend(iter_2);
        } else {
            // Neither sequence has dynamic end, so we can just extend with the rest.
            // Note that one of these will be empty.
            res.extend(iter_1);
            res.extend(iter_2);
        }

        Sequence {
            elements: res,
            end_dynamic,
        }
    }

    pub fn index_into(&self, mut index: usize) -> Option<Type> {
        for elem in &self.elements {
            if index == 0 {
                return Some(elem.clone());
            }
            index -= 1;
        }
        if let Some(end_dynamic) = &self.end_dynamic {
            return Some(end_dynamic.as_ref().clone());
        }
        None
    }

    pub fn index_from_end(&self, mut index: usize) -> Option<Type> {
        if let Some(end_dynamic) = &self.end_dynamic {
            return Some(end_dynamic.as_ref().clone());
        }

        for elem in self.elements.iter().rev() {
            if index == 0 {
                return Some(elem.clone());
            }
            index -= 1;
        }
        None
    }

    pub fn element_union(&self) -> Type {
        let mut ty = Type::Union(Vec::new());
        for elem in &self.elements {
            ty = ty.union_with(elem.clone());
        }
        if let Some(end_dynamic) = &self.end_dynamic {
            ty = ty.union_with(end_dynamic.as_ref().clone());
        }
        ty
    }

    pub fn all_elements(&self) -> impl Iterator<Item = &Type> {
        self.elements
            .iter()
            .chain(self.end_dynamic.iter().map(|d| d.as_ref()))
    }

    pub fn can_be_assigned_from(&self, other: &[Value]) -> bool {
        let mut other_iter = other.iter();

        for self_elem in self.elements.iter() {
            let Some(other_elem) = other_iter.next() else {
                // If we run out of elements, it means the other sequence is too short.
                return false;
            };

            if !Type::from_const(other_elem.clone()).is_assignable_to(&self_elem) {
                return false;
            }
        }

        if let Some(end_dynamic) = &self.end_dynamic {
            for elem in other_iter {
                if !Type::from_const(elem.clone()).is_assignable_to(&end_dynamic) {
                    return false;
                }
            }
        }
        true
    }

    pub fn can_be_assigned_to(&self, other: &[Value]) -> bool {
        let mut other_iter = other.iter();

        for self_elem in self.elements.iter() {
            let Some(other_elem) = other_iter.next() else {
                // If we run out of elements, it means the other sequence is too short.
                return false;
            };

            if !self_elem.is_assignable_to(&Type::from_const(other_elem.clone())) {
                return false;
            }
        }

        if let Some(end_dynamic) = &self.end_dynamic {
            for elem in other_iter {
                if !end_dynamic.is_assignable_to(&Type::from_const(elem.clone())) {
                    return false;
                }
            }
            true
        } else {
            other_iter.next().is_none()
        }
    }

    pub fn is_assignable_to(&self, other: &Sequence) -> bool {
        let mut self_iter = self.elements.iter();
        let mut other_iter = other.elements.iter();

        loop {
            let Some(self_elem) = self_iter.next() else {
                // If we run out of elements in self, all remaining elements in other must be
                // compatible with the end dynamic of self.
                if let Some(end_dynamic) = &self.end_dynamic {
                    for other_elem in other_iter {
                        if !end_dynamic.is_assignable_to(other_elem) {
                            return false;
                        }
                    }
                    return true;
                } else {
                    // If self has no end dynamic, and there are still elements in other,
                    // it means that they are not compatible.
                    return other_iter.next().is_none();
                }
            };
            let Some(other_elem) = other_iter.next() else {
                // If we run out of elements in other, all remaining elements in self must be
                // compatible with the end dynamic of other.
                if let Some(end_dynamic) = &other.end_dynamic {
                    if !self_elem.is_assignable_to(end_dynamic) {
                        return false;
                    }
                    for self_elem in self_iter {
                        if !self_elem.is_assignable_to(end_dynamic) {
                            return false;
                        }
                    }
                    return true;
                } else {
                    // If other has no end dynamic, it means it is not compatible with any remaining
                    // elements in self.
                    return false;
                }
            };

            if !self_elem.is_assignable_to(other_elem) {
                return false;
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("Expected {0} but got {1}")]
    ExpectedType(Type, Type, Span),
}

#[derive(Debug, Clone, Copy)]
pub enum Truthy {
    Always,
    Never,
    Maybe,
}

impl From<bool> for Type {
    fn from(value: bool) -> Self {
        Self::Constant(Value::Bool(value.into()))
    }
}

impl Type {
    pub fn union_with(self, other: Type) -> Self {
        match (self, other) {
            (Type::Union(mut types), Type::Union(other_types)) => {
                types.extend(other_types);
                Type::Union(types)
            }
            (Type::Any, _) => Type::Any,
            (_, Type::Any) => Type::Any,
            (Type::Union(mut types), other) | (other, Type::Union(mut types)) => {
                if !types.contains(&other) {
                    types.push(other);
                }
                // For sanity, merge objects and sequences.
                else if let Type::Object(obj) = other {
                    for typ in &mut types {
                        if let Type::Object(obj2) = typ {
                            *typ = Type::Object(obj.union_with(obj2.clone()));
                            return Type::Union(types);
                        }
                    }
                    types.push(Type::Object(obj));
                } else if let Type::Sequence(seq) = other {
                    for typ in &mut types {
                        if let Type::Sequence(seq2) = typ {
                            *typ = Type::Sequence(seq.union_with(seq2.clone()));
                            return Type::Union(types);
                        }
                    }
                    types.push(Type::Sequence(seq));
                }
                Type::Union(types)
            }

            (Type::Sequence(seq1), Type::Sequence(seq2)) => Type::Sequence(seq1.union_with(seq2)),
            (Type::Object(obj1), Type::Object(obj2)) => Type::Object(obj1.union_with(obj2)),
            (self_type, other_type) => {
                if self_type == other_type {
                    self_type
                } else {
                    Type::Union(vec![self_type, other_type])
                }
            }
        }
    }

    pub fn from_const(value: Value) -> Self {
        Type::Constant(value)
    }

    pub fn any_array() -> Self {
        Type::Sequence(Sequence {
            elements: vec![],
            end_dynamic: Some(Box::new(Type::Any)),
        })
    }

    pub fn any_object() -> Self {
        Type::Object(Object {
            fields: [(ObjectField::Generic, Type::Any)].into_iter().collect(),
        })
    }

    pub fn try_as_array(&self, span: &Span) -> Result<Sequence, TypeError> {
        match self {
            Type::Sequence(seq) => Ok(seq.clone()),
            Type::Union(types) => {
                let seq = types
                    .iter()
                    .filter_map(|t| {
                        if let Type::Sequence(seq) = t {
                            Some(seq.clone())
                        } else {
                            None
                        }
                    })
                    .next();
                let Some(seq) = seq else {
                    return Err(TypeError::ExpectedType(
                        Type::any_array(),
                        self.clone(),
                        span.clone(),
                    ));
                };
                Ok(seq)
            }
            Type::Constant(Value::Array(arr)) => {
                let elements = arr.iter().map(|v| Type::from_const(v.clone())).collect();
                Ok(Sequence {
                    elements,
                    end_dynamic: None,
                })
            }
            Type::Any => Ok(Sequence {
                end_dynamic: Some(Box::new(Type::Any)),
                elements: vec![],
            }),
            _ => Err(TypeError::ExpectedType(
                Type::any_array(),
                self.clone(),
                span.clone(),
            )),
        }
    }

    pub fn try_as_object(&self, span: &Span) -> Result<Object, TypeError> {
        match self {
            Type::Object(obj) => Ok(obj.clone()),
            Type::Union(types) => {
                let obj = types
                    .iter()
                    .filter_map(|t| {
                        if let Type::Object(obj) = t {
                            Some(obj.clone())
                        } else {
                            None
                        }
                    })
                    .next();
                let Some(obj) = obj else {
                    return Err(TypeError::ExpectedType(
                        Type::any_object(),
                        self.clone(),
                        span.clone(),
                    ));
                };
                Ok(obj)
            }
            Type::Constant(Value::Object(obj)) => {
                let fields = obj
                    .iter()
                    .map(|(k, v)| {
                        (
                            ObjectField::Constant(k.clone()),
                            Type::from_const(v.clone()),
                        )
                    })
                    .collect();
                Ok(Object { fields })
            }
            Type::Any => Ok(Object {
                fields: [(ObjectField::Generic, Type::Any)].into_iter().collect(),
            }),
            _ => Err(TypeError::ExpectedType(
                Type::any_object(),
                self.clone(),
                span.clone(),
            )),
        }
    }

    pub fn truthyness(&self) -> Truthy {
        match self {
            Type::Constant(Value::Null | Value::Bool(false)) => Truthy::Never,
            Type::Constant(_) => Truthy::Always,
            Type::Object(_) => Truthy::Always,
            Type::Sequence(_) => Truthy::Always,
            Type::String => Truthy::Always,
            Type::Integer => Truthy::Always,
            Type::Float => Truthy::Always,
            Type::Boolean => Truthy::Maybe,
            Type::Union(items) => {
                let mut t = None;
                for item in items {
                    match (t, item.truthyness()) {
                        (None, r) => t = Some(r),
                        (_, Truthy::Maybe) | (Some(Truthy::Maybe), _) => return Truthy::Maybe,
                        (Some(Truthy::Always), Truthy::Always) => continue,
                        (Some(Truthy::Never), Truthy::Never) => continue,
                        (Some(Truthy::Always), Truthy::Never)
                        | (Some(Truthy::Never), Truthy::Always) => return Truthy::Maybe,
                    }
                }
                // Should technically be unreachable, an empty union is a never type.
                let Some(t) = t else {
                    return Truthy::Never;
                };
                t
            }
            Type::Any => Truthy::Maybe,
        }
    }

    pub fn assert_assignable_to(&self, key: &Type, span: &Span) -> Result<(), TypeError> {
        if !self.is_assignable_to(&key) {
            Err(TypeError::ExpectedType(
                key.clone(),
                self.clone(),
                span.clone(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn is_assignable_to(&self, other: &Type) -> bool {
        match (self, other) {
            // Any type can be assigned to Any and Any can be assigned to any type.
            (Type::Any, _) | (_, Type::Any) => true,
            // Constants can be assigned to any constant...
            (Type::Constant(v1), Type::Constant(v2)) => v1 == v2,
            // ...or to a generic type of the same kind.
            (Type::Constant(v1), Type::String) | (Type::String, Type::Constant(v1)) => {
                v1.is_string()
            }
            (Type::Constant(v1), Type::Integer) | (Type::Integer, Type::Constant(v1)) => {
                v1.is_i64() || v1.is_u64()
            }
            (Type::Constant(v1), Type::Float) | (Type::Float, Type::Constant(v1)) => v1.is_f64(),
            (Type::Constant(v1), Type::Boolean) | (Type::Boolean, Type::Constant(v1)) => {
                v1.is_boolean()
            }
            // Assignable if all fields of the object are accepted by the object type.
            (Type::Constant(v1), Type::Object(v2)) => {
                let Some(obj) = v1.as_object() else {
                    return false;
                };
                for (key, value) in obj {
                    if !v2.accepts_field(key, &Type::from_const(value.clone())) {
                        return false;
                    }
                }
                true
            }
            (Type::Object(v1), Type::Constant(v2)) => {
                let Some(obj) = v2.as_object() else {
                    return false;
                };
                for (key, value) in obj {
                    if !v1.can_have_field(key, &Type::from_const(value.clone())) {
                        return false;
                    }
                }
                true
            }
            // Assignable to a sequence by iterating through the sequences together,
            // until we hit the first dynamic element.
            (Type::Constant(v1), Type::Sequence(v2)) => {
                let Some(arr) = v1.as_array() else {
                    return false;
                };
                v2.can_be_assigned_from(arr.as_slice())
            }
            (Type::Sequence(v1), Type::Constant(v2)) => {
                let Some(arr) = v2.as_array() else {
                    return false;
                };
                v1.can_be_assigned_to(arr.as_slice())
            }
            // A union can be assigned to some other type if at least one of its types
            // can be assigned to the other type.
            (Type::Union(types), other) => types.iter().any(|t| t.is_assignable_to(other)),
            (other, Type::Union(types)) => types.iter().any(|t| other.is_assignable_to(t)),
            (Type::Sequence(v1), Type::Sequence(v2)) => v1.is_assignable_to(v2),
            (Type::Object(v1), Type::Object(v2)) => v1.is_assignable_to(v2),
            // Fall back to exact equality.
            (v1, v2) => v1 == v2,
        }
    }

    pub fn number() -> Self {
        Type::Union(vec![Type::Integer, Type::Float])
    }

    pub fn null() -> Self {
        Type::Constant(Value::Null)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Type::Constant(Value::Null))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Type::Boolean | Type::Constant(Value::Bool(_)))
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Type::Integer => true,
            Type::Constant(Value::Number(n)) => n.is_u64() || n.is_u64(),
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            Type::Float => true,
            Type::Constant(Value::Number(n)) => n.is_f64(),
            _ => false,
        }
    }

    pub fn never() -> Self {
        Type::Union(Vec::new())
    }

    pub fn is_never(&self) -> bool {
        match self {
            Type::Union(types) => types.is_empty(),
            _ => false,
        }
    }

    pub fn const_equals(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Constant(v1), Type::Constant(v2)) => v1 == v2,
            _ => false,
        }
    }

    pub fn flatten_union(self) -> Self {
        if let Type::Union(types) = self {
            if types.len() == 1 {
                return types.into_iter().next().unwrap();
            }
            Type::Union(types)
        } else {
            self
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Constant(value) => write!(f, "{}", value),
            Type::Object(object) => {
                write!(f, "{{")?;
                let mut needs_comma = false;
                for (key, value) in &object.fields {
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
            Type::Sequence(sequence) => {
                write!(f, "[")?;
                let mut needs_comma = false;
                for elem in &sequence.elements {
                    if needs_comma {
                        write!(f, ", ")?;
                    } else {
                        needs_comma = true;
                    }
                    write!(f, "{}", elem)?;
                }
                if let Some(end_dynamic) = &sequence.end_dynamic {
                    if needs_comma {
                        write!(f, ", ")?;
                    }
                    write!(f, "...{}", end_dynamic)?;
                }
                write!(f, "]")
            }
            Type::String => write!(f, "String"),
            Type::Integer => write!(f, "Integer"),
            Type::Float => write!(f, "Float"),
            Type::Boolean => write!(f, "Boolean"),
            Type::Union(items) => {
                write!(f, "Union<")?;
                let mut needs_comma = false;
                for item in items {
                    if needs_comma {
                        write!(f, ", ")?;
                    } else {
                        needs_comma = true;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ">")
            }
            Type::Any => write!(f, "Any"),
        }
    }
}

pub struct TypeExecutionState<'data, 'exec> {
    data: &'exec Vec<&'data Type>,
}

static NULL_TYPE_CONST: Type = Type::Constant(Value::Null);

impl<'data, 'exec> TypeExecutionState<'data, 'exec> {
    pub fn new(data: &'exec Vec<&'data Type>) -> Self {
        Self { data }
    }

    pub fn get_type(&self, index: usize) -> Option<&'data Type> {
        self.data.get(index).cloned()
    }

    pub fn get_temporary_clone<'inner>(
        &'inner mut self,
        extra_types: impl Iterator<Item = &'inner Type>,
        num_values: usize,
    ) -> InternalTypeExecutionState<'inner>
    where
        'data: 'inner,
    {
        let mut data = Vec::with_capacity(self.data.len() + num_values);
        for elem in self.data.iter() {
            data.push(*elem);
        }
        let mut pushed = 0;
        for elem in extra_types.take(num_values) {
            data.push(elem);
            pushed += 1;
        }
        if pushed < num_values {
            for _ in pushed..num_values {
                data.push(&NULL_TYPE_CONST);
            }
        }

        InternalTypeExecutionState { data }
    }
}

pub struct InternalTypeExecutionState<'data> {
    data: Vec<&'data Type>,
}

impl<'data> InternalTypeExecutionState<'data> {
    pub fn get_temp_state<'slf>(&'slf mut self) -> TypeExecutionState<'data, 'slf> {
        TypeExecutionState::new(&self.data)
    }
}
