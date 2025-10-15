use std::fmt::Display;

use logos::Span;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

mod array;
mod object;

pub use array::Array;
pub use object::{Object, ObjectField};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Representation of a type in the Kuiper language.
/// Since Kuiper is dynamically typed, this needs to represent any
/// JSON value.
pub enum Type {
    /// A constant JSON value. I.e. a value literal.
    Constant(Value),

    // TODO
    /// Some JSON object.
    Object(Object),
    /// Some JSON array.
    Array(Array),
    /// Some string.
    String,
    /// Some integer number.
    Integer,
    /// Some floating point number.
    Float,
    /// Some boolean value.
    Boolean,
    /// A union of multiple types. If constructed using
    /// methods on `Type`, this will not contain unions or duplicates.
    /// `Union(Vec::new())` represents the never type.
    Union(Vec<Type>),
    /// Any type, technically the same as a union of all possible types.
    Any,
}

#[derive(Debug, Error)]
/// An error returned during type checking.
pub enum TypeError {
    /// The intersection of the provided type (1) and the expected type (0)
    /// is empty, meaning that the types are guaranteed to be incompatible.
    #[error("Expected {0} but got {1}")]
    ExpectedType(Box<Type>, Box<Type>, Span),
}

impl TypeError {
    /// Get the span of the type error.
    pub fn span(&self) -> &Span {
        match self {
            TypeError::ExpectedType(_, _, span) => span,
        }
    }

    pub fn expected_type(expected: Type, got: Type, span: Span) -> Self {
        TypeError::ExpectedType(Box::new(expected), Box::new(got), span)
    }
}

impl Type {
    /// Create a new constant type.
    pub fn from_const(value: impl Into<Value>) -> Self {
        Type::Constant(value.into())
    }

    /// Create a new type that can be either number, i.e. either integer or float.
    pub fn number() -> Self {
        Type::Union(vec![Type::Integer, Type::Float])
    }

    /// Create a new null type, this is a constant type with the null value.
    pub fn null() -> Self {
        Type::Constant(Value::Null)
    }

    /// Check whether the type is equal to the null type.
    pub fn is_null(&self) -> bool {
        matches!(self, Type::Constant(Value::Null))
    }

    /// Check whether the is a boolean.
    pub fn is_boolean(&self) -> bool {
        matches!(self, Type::Boolean | Type::Constant(Value::Bool(_)))
    }

    /// Check whether the type is an integer.
    pub fn is_integer(&self) -> bool {
        match self {
            Type::Integer => true,
            Type::Constant(Value::Number(n)) => n.is_u64() || n.is_u64(),
            _ => false,
        }
    }

    /// Check whether the type is a floating point number.
    pub fn is_float(&self) -> bool {
        match self {
            Type::Float => true,
            Type::Constant(Value::Number(n)) => n.is_f64(),
            _ => false,
        }
    }

    /// Create a new never type, which is a union of no types.
    /// If the result of an expression is the never type, it means that
    /// the expression will never successfully complete.
    pub fn never() -> Self {
        Type::Union(Vec::new())
    }

    /// Check whether the type is the never type.
    pub fn is_never(&self) -> bool {
        match self {
            Type::Union(types) => types.is_empty(),
            _ => false,
        }
    }

    /// Check whether the other type is always equal to this type,
    /// meaning they are both constant and have the same value.
    /// If either type is not constant, this will return false.
    pub fn const_equals(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Constant(v1), Type::Constant(v2)) => v1 == v2,
            _ => false,
        }
    }

    /// If this is a union with a single possible type, return that type.
    /// Otherwise return self.
    /// This is useful to simplify types after operations that may
    /// produce unions.
    pub fn extract_single_union(self) -> Self {
        match self {
            Type::Union(types) if types.len() == 1 => types.into_iter().next().unwrap(),
            _ => self,
        }
    }

    /// Return a type representing any object, i.e. an object with no known fields,
    /// and where unknown fields are of type `Any`.
    pub fn any_object() -> Self {
        Self::object_of_type(Type::Any)
    }

    /// Return a type representing an object where all fields are of the given type.
    pub fn object_of_type(field_type: Type) -> Self {
        Type::Object(Object {
            fields: [(ObjectField::Generic, field_type)].into_iter().collect(),
        })
    }

    /// Return a type representing any array, i.e. an array where all elements are of type `Any`.
    pub fn any_array() -> Self {
        Type::array_of_type(Type::Any)
    }

    /// Return a type representing an array where all elements are of the given type.
    pub fn array_of_type(field_type: Type) -> Self {
        Type::Array(Array {
            elements: Vec::new(),
            end_dynamic: Some(Box::new(field_type)),
        })
    }

    /// Try to treat this type as an object type, by inspecting it if
    /// it is a union, a constant, or any. This will return an error if
    /// the type
    ///
    ///  - Is a union without a variant that can be treated as an object.
    ///  - Is a constant that is not an object.
    ///  - Is not an object, union, constant, or any.
    ///
    /// Note that this assumes that if it is a union, this union is normalized, i.e.
    /// contains only types that are not themselves unions, does not contain duplicates,
    /// and does not contain `Any`.
    pub fn try_as_object(&self, span: &Span) -> Result<Object, TypeError> {
        match self {
            Type::Object(obj) => Ok(obj.clone()),
            Type::Union(types) => {
                // TODO: This currently just picks the first object it finds,
                // we likely want to compute the union of all possible objects instead.
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
                    return Err(TypeError::expected_type(
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
            _ => Err(TypeError::expected_type(
                Type::any_object(),
                self.clone(),
                span.clone(),
            )),
        }
    }

    /// Try to treat this type as an array type, by inspecting it if
    /// it is a union, a constant, or any. This will return an error if
    /// the type
    ///
    ///  - Is a union without a variant that can be treated as an array.
    ///  - Is a constant that is not an array.
    ///  - Is not an array, union, constant, or any.
    ///
    /// Note that this assumes that if it is a union, this union is normalized, i.e.
    /// contains only types that are not themselves unions, does not contain duplicates,
    /// and does not contain `Any`.
    pub fn try_as_array(&self, span: &Span) -> Result<Array, TypeError> {
        match self {
            Type::Array(seq) => Ok(seq.clone()),
            Type::Union(types) => {
                // TODO: This currently just picks the first array it finds,
                // we likely want to compute the union of all possible arrays instead.
                let seq = types
                    .iter()
                    .filter_map(|t| {
                        if let Type::Array(seq) = t {
                            Some(seq.clone())
                        } else {
                            None
                        }
                    })
                    .next();
                let Some(seq) = seq else {
                    return Err(TypeError::expected_type(
                        Type::any_array(),
                        self.clone(),
                        span.clone(),
                    ));
                };
                Ok(seq)
            }
            Type::Constant(Value::Array(arr)) => {
                let elements = arr.iter().map(|v| Type::from_const(v.clone())).collect();
                Ok(Array {
                    elements,
                    end_dynamic: None,
                })
            }
            Type::Any => Ok(Array {
                end_dynamic: Some(Box::new(Type::Any)),
                elements: vec![],
            }),
            _ => Err(TypeError::expected_type(
                Type::any_array(),
                self.clone(),
                span.clone(),
            )),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Constant(value) => write!(f, "{}", value),
            Type::Object(o) => write!(f, "{o}"),
            Type::Array(s) => write!(f, "{s}"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_const_equals() {
        let t1 = Type::from_const(json!(42));
        let t2 = Type::from_const(json!(42));
        let t3 = Type::from_const(json!(43));
        let t4 = Type::Integer;

        assert!(t1.const_equals(&t2));
        assert!(!t1.const_equals(&t3));
        assert!(!t1.const_equals(&t4));
    }

    #[test]
    fn test_is_integer() {
        let t1 = Type::Integer;
        let t2 = Type::from_const(json!(42));
        let t3 = Type::from_const(json!(42.0));
        let t4 = Type::Float;

        assert!(t1.is_integer());
        assert!(t2.is_integer());
        assert!(!t3.is_integer());
        assert!(!t4.is_integer());
    }

    #[test]
    fn test_is_float() {
        let t1 = Type::Float;
        let t2 = Type::from_const(json!(42.0));
        let t3 = Type::from_const(json!(42));
        let t4 = Type::Integer;

        assert!(t1.is_float());
        assert!(t2.is_float());
        assert!(!t3.is_float());
        assert!(!t4.is_float());
    }

    #[test]
    fn test_is_boolean() {
        let t1 = Type::Boolean;
        let t2 = Type::from_const(json!(true));
        let t3 = Type::from_const(json!(false));
        let t4 = Type::from_const(json!(42));

        assert!(t1.is_boolean());
        assert!(t2.is_boolean());
        assert!(t3.is_boolean());
        assert!(!t4.is_boolean());
    }

    #[test]
    fn test_is_null() {
        let t1 = Type::null();
        let t2 = Type::from_const(json!(null));
        let t3 = Type::from_const(json!(42));

        assert!(t1.is_null());
        assert!(t2.is_null());
        assert!(!t3.is_null());
    }

    #[test]
    fn test_never_type() {
        let never_type = Type::never();
        let non_never_type = Type::Union(vec![Type::Integer]);

        assert!(never_type.is_never());
        assert!(!non_never_type.is_never());
    }

    #[test]
    fn test_extract_single_union() {
        let single_union = Type::Union(vec![Type::Integer]);
        let multi_union = Type::Union(vec![Type::Integer, Type::Float]);
        let non_union = Type::String;
        let never = Type::never();

        assert_eq!(single_union.extract_single_union(), Type::Integer);
        assert_eq!(multi_union.clone().extract_single_union(), multi_union);
        assert_eq!(non_union.clone().extract_single_union(), non_union);
        assert_eq!(never.clone().extract_single_union(), never);
    }

    #[test]
    fn test_display() {
        let t1 = Type::from_const(json!(42));
        let t2 = Type::String;
        let t3 = Type::Union(vec![Type::Integer, Type::Float]);
        let t4 = Type::Any;
        let t5 = Type::never();
        let t6 = Type::Boolean;
        let t7 = Type::from_const(json!(true));
        let t8 = Type::from_const(json!(null));
        let t9 = Type::Union(vec![Type::from_const(json!(1)), Type::from_const(json!(2))]);
        let t10 = Type::Union(vec![Type::from_const(json!(1)), Type::Integer]);
        let t11 = Type::Object(Object {
            fields: [
                (ObjectField::Constant("a".to_string()), Type::String),
                (ObjectField::Generic, Type::Integer),
            ]
            .into_iter()
            .collect(),
        });
        let t12 = Type::Array(Array {
            elements: vec![Type::String, Type::Integer],
            end_dynamic: Some(Box::new(Type::Float)),
        });
        assert_eq!(t1.to_string(), "42");
        assert_eq!(t2.to_string(), "String");
        assert_eq!(t3.to_string(), "Union<Integer, Float>");
        assert_eq!(t4.to_string(), "Any");
        assert_eq!(t5.to_string(), "Union<>");
        assert_eq!(t6.to_string(), "Boolean");
        assert_eq!(t7.to_string(), "true");
        assert_eq!(t8.to_string(), "null");
        assert_eq!(t9.to_string(), "Union<1, 2>");
        assert_eq!(t10.to_string(), "Union<1, Integer>");
        assert_eq!(t11.to_string(), "{a: String, ...: Integer}");
        assert_eq!(t12.to_string(), "[String, Integer, ...Float]");
    }

    #[test]
    fn test_type_execution_state() {
        let t1 = Type::Integer;
        let t2 = Type::Float;
        let t3 = Type::String;

        let types = vec![&t1, &t2];
        let mut state = TypeExecutionState::new(&types);

        assert_eq!(state.get_type(0), Some(&Type::Integer));
        assert_eq!(state.get_type(1), Some(&Type::Float));
        assert_eq!(state.get_type(2), None);

        let extra_types = vec![&t3];
        let mut internal_state = state.get_temporary_clone(extra_types.into_iter(), 2);
        let temp_state = internal_state.get_temp_state();

        assert_eq!(temp_state.get_type(0), Some(&Type::Integer));
        assert_eq!(temp_state.get_type(1), Some(&Type::Float));
        assert_eq!(temp_state.get_type(2), Some(&Type::String));
        assert_eq!(temp_state.get_type(3), Some(&Type::Constant(Value::Null)));
    }

    #[test]
    fn test_try_as_object() {
        let obj_type = Type::Object(Object {
            fields: [
                (ObjectField::Constant("a".to_string()), Type::String),
                (ObjectField::Generic, Type::Integer),
            ]
            .into_iter()
            .collect(),
        });
        let union_type = Type::Union(vec![
            Type::Object(Object {
                fields: [(ObjectField::Constant("b".to_string()), Type::Float)]
                    .into_iter()
                    .collect(),
            }),
            Type::String,
        ]);
        let const_obj_type = Type::from_const(json!({"c": 42, "d": "hello"}));
        let any_type = Type::Any;
        let non_obj_type = Type::Integer;

        let span = 0..1;

        // Test direct object type
        let result = obj_type.try_as_object(&span).unwrap();
        assert_eq!(
            result.fields.get(&ObjectField::Constant("a".to_string())),
            Some(&Type::String)
        );
        assert_eq!(
            result.fields.get(&ObjectField::Generic),
            Some(&Type::Integer)
        );

        // Test union type with an object variant
        let result = union_type.try_as_object(&span).unwrap();
        assert_eq!(
            result.fields.get(&ObjectField::Constant("b".to_string())),
            Some(&Type::Float)
        );
        assert!(result.fields.get(&ObjectField::Generic).is_none());

        // Test constant object type
        let result = const_obj_type.try_as_object(&span).unwrap();
        assert_eq!(
            result.fields.get(&ObjectField::Constant("c".to_string())),
            Some(&Type::from_const(42))
        );
        assert_eq!(
            result.fields.get(&ObjectField::Constant("d".to_string())),
            Some(&Type::from_const("hello"))
        );
        assert!(result.fields.get(&ObjectField::Generic).is_none());

        // Test Any type
        let result = any_type.try_as_object(&span).unwrap();
        assert_eq!(result.fields.get(&ObjectField::Generic), Some(&Type::Any));

        // Test non-object type
        let err = non_obj_type.try_as_object(&span).unwrap_err();
        match err {
            TypeError::ExpectedType(expected, got, _) => {
                assert_eq!(
                    expected,
                    Box::new(Type::Object(Object {
                        fields: [(ObjectField::Generic, Type::Any)].into_iter().collect(),
                    }))
                );
                assert_eq!(*got, Type::Integer);
            }
        }
    }

    #[test]
    fn test_try_as_object_union_without_object() {
        let union_type = Type::Union(vec![Type::String, Type::Integer]);
        let span = 0..1;

        let err = union_type.try_as_object(&span).unwrap_err();
        match err {
            TypeError::ExpectedType(expected, got, _) => {
                assert_eq!(
                    expected,
                    Box::new(Type::Object(Object {
                        fields: [(ObjectField::Generic, Type::Any)].into_iter().collect(),
                    }))
                );
                assert_eq!(*got, union_type);
            }
        }
    }

    #[test]
    fn test_try_as_array() {
        let arr_type = Type::Array(Array {
            elements: vec![Type::String, Type::Integer],
            end_dynamic: Some(Box::new(Type::Float)),
        });
        let union_type = Type::Union(vec![
            Type::Array(Array {
                elements: vec![Type::Boolean],
                end_dynamic: None,
            }),
            Type::String,
        ]);
        let const_arr_type = Type::from_const(json!(["hello", 42, 3.14]));
        let any_type = Type::Any;
        let non_arr_type = Type::Integer;

        let span = 0..1;

        // Test direct array type
        let result = arr_type.try_as_array(&span).unwrap();
        assert_eq!(result.elements.len(), 2);
        assert_eq!(result.elements[0], Type::String);
        assert_eq!(result.elements[1], Type::Integer);
        assert_eq!(*result.end_dynamic.unwrap(), Type::Float);

        // Test union type with an array variant
        let result = union_type.try_as_array(&span).unwrap();
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0], Type::Boolean);
        assert!(result.end_dynamic.is_none());

        // Test constant array type
        let result = const_arr_type.try_as_array(&span).unwrap();
        assert_eq!(result.elements.len(), 3);
        assert_eq!(result.elements[0], Type::from_const("hello"));
        assert_eq!(result.elements[1], Type::from_const(42));
        assert_eq!(result.elements[2], Type::from_const(3.14));
        assert!(result.end_dynamic.is_none());

        // Test Any type
        let result = any_type.try_as_array(&span).unwrap();
        assert!(result.elements.is_empty());
        assert_eq!(*result.end_dynamic.unwrap(), Type::Any);

        // Test non-array type
        let err = non_arr_type.try_as_array(&span).unwrap_err();
        match err {
            TypeError::ExpectedType(expected, got, _) => {
                assert_eq!(
                    expected,
                    Box::new(Type::Array(Array {
                        elements: vec![],
                        end_dynamic: Some(Box::new(Type::Any)),
                    }))
                );
                assert_eq!(*got, Type::Integer);
            }
        }
    }

    #[test]
    fn test_try_as_array_union_without_array() {
        let union_type = Type::Union(vec![Type::String, Type::Integer]);
        let span = 0..1;
        let err = union_type.try_as_array(&span).unwrap_err();
        match err {
            TypeError::ExpectedType(expected, got, _) => {
                assert_eq!(
                    expected,
                    Box::new(Type::Array(Array {
                        elements: vec![],
                        end_dynamic: Some(Box::new(Type::Any)),
                    }))
                );
                assert_eq!(*got, union_type);
            }
        }
    }
}
