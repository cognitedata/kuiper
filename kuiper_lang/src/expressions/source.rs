use std::fmt::Debug;

use crate::{expressions::ResolveResult, NULL_CONST};

/// A trait representing a data source for an expression. This must allow returning values as JSON dynamically.
/// It is implemented for `serde_json::Value` so that JSON data can be used directly. For convenience it is
/// also implemented for a handful of standard library types, such as `Vec`, `Option` and `HashMap`.
///
/// When implementing this trait for custom types, it is recommended to use `NULL_CONST`
/// as a fallback for missing keys or indices, though in practice this can be used to provide any kind
/// of custom input behavior, if needed.
///
/// The primary purpose of this is to avoid allocations when providing complex context data to expressions.
/// If allocations are not a concern, using `serde_json::Value` directly is simpler.
pub trait SourceData: Debug {
    /// Get the current value as a JSON value.
    fn resolve(&self) -> ResolveResult<'_>;
    /// Get a value by key, returning a reference to another `SourceData`.
    fn get_key(&self, _key: &str) -> &dyn SourceData {
        &NULL_CONST
    }
    /// Get a value by index in an array, returning a reference to another `SourceData`.
    fn get_index(&self, _index: usize) -> &dyn SourceData {
        &NULL_CONST
    }
    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(std::iter::empty())
    }
    fn len(&self) -> Option<usize> {
        None
    }
    fn is_null(&self) -> bool {
        false
    }
}

impl SourceData for serde_json::Value {
    fn resolve(&self) -> ResolveResult<'_> {
        ResolveResult::Borrowed(self)
    }

    fn get_key(&self, key: &str) -> &dyn SourceData {
        match self {
            serde_json::Value::Object(map) => map.get(key).unwrap_or(&NULL_CONST),
            _ => &NULL_CONST,
        }
    }

    fn get_index(&self, index: usize) -> &dyn SourceData {
        match self {
            serde_json::Value::Array(arr) => arr.get(index).unwrap_or(&NULL_CONST),
            _ => &NULL_CONST,
        }
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match self {
            serde_json::Value::Object(map) => Box::new(map.keys().map(|k| k.as_str())),
            _ => Box::new(std::iter::empty()),
        }
    }

    fn len(&self) -> Option<usize> {
        match self {
            serde_json::Value::Array(arr) => Some(arr.len()),
            serde_json::Value::Object(map) => Some(map.len()),
            _ => None,
        }
    }

    fn is_null(&self) -> bool {
        self.is_null()
    }
}

impl<T> SourceData for &T
where
    T: SourceData + ?Sized,
{
    fn resolve(&self) -> ResolveResult<'_> {
        (*self).resolve()
    }

    fn get_key(&self, key: &str) -> &dyn SourceData {
        (*self).get_key(key)
    }

    fn get_index(&self, index: usize) -> &dyn SourceData {
        (*self).get_index(index)
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        (*self).keys()
    }

    fn len(&self) -> Option<usize> {
        (*self).len()
    }

    fn is_null(&self) -> bool {
        (*self).is_null()
    }
}

impl<T> SourceData for Box<T>
where
    T: SourceData + ?Sized,
{
    fn resolve(&self) -> ResolveResult<'_> {
        (**self).resolve()
    }

    fn get_key(&self, key: &str) -> &dyn SourceData {
        (**self).get_key(key)
    }

    fn get_index(&self, index: usize) -> &dyn SourceData {
        (**self).get_index(index)
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        (**self).keys()
    }

    fn len(&self) -> Option<usize> {
        (**self).len()
    }

    fn is_null(&self) -> bool {
        (**self).is_null()
    }
}

impl<T> SourceData for Vec<T>
where
    T: SourceData,
{
    fn resolve(&self) -> ResolveResult<'_> {
        ResolveResult::Owned(serde_json::Value::Array(
            self.iter().map(|v| v.resolve().into_owned()).collect(),
        ))
    }

    fn get_index(&self, index: usize) -> &dyn SourceData {
        self.get(index)
            .map(|v| v as &dyn SourceData)
            .unwrap_or(&NULL_CONST)
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<T> SourceData for Option<T>
where
    T: SourceData,
{
    fn resolve(&self) -> ResolveResult<'_> {
        match self {
            Some(v) => v.resolve(),
            None => ResolveResult::Owned(serde_json::Value::Null),
        }
    }

    fn get_key(&self, key: &str) -> &dyn SourceData {
        match self {
            Some(v) => v.get_key(key),
            None => &NULL_CONST,
        }
    }

    fn get_index(&self, index: usize) -> &dyn SourceData {
        match self {
            Some(v) => v.get_index(index),
            None => &NULL_CONST,
        }
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match self {
            Some(v) => v.keys(),
            None => Box::new(std::iter::empty()),
        }
    }

    fn len(&self) -> Option<usize> {
        match self {
            Some(v) => v.len(),
            None => None,
        }
    }

    fn is_null(&self) -> bool {
        self.is_none()
    }
}

impl<T> SourceData for std::collections::HashMap<String, T>
where
    T: SourceData,
{
    fn resolve(&self) -> ResolveResult<'_> {
        let map: serde_json::Map<String, serde_json::Value> = self
            .iter()
            .map(|(k, v)| (k.clone(), v.resolve().into_owned()))
            .collect();
        ResolveResult::Owned(serde_json::Value::Object(map))
    }

    fn get_key(&self, key: &str) -> &dyn SourceData {
        self.get(key)
            .map(|v| v as &dyn SourceData)
            .unwrap_or(&NULL_CONST)
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.keys().map(|k| k.as_str()))
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }
}

macro_rules! impl_source_for_primitive {
    ($($t:ty),*) => {
        $(
            impl SourceData for $t {
                fn resolve(&self) -> ResolveResult<'_> {
                    ResolveResult::Owned(serde_json::Value::from(*self))
                }
            }
        )*
    };
    ([clone] $($t:ty),*) => {
        $(
            impl SourceData for $t {
                fn resolve(&self) -> ResolveResult<'_> {
                    ResolveResult::Owned(serde_json::Value::from(self.clone()))
                }
            }
        )*
    };
}

impl_source_for_primitive!(bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, &str, usize);
impl_source_for_primitive!([clone] String, serde_json::Number);

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use crate::{
        compile_expression,
        expressions::{source::SourceData, ResolveResult},
    };

    #[derive(Debug, Serialize)]
    struct CustomData {
        name: String,
        values: Vec<i32>,
    }

    impl SourceData for CustomData {
        fn resolve(&self) -> crate::expressions::ResolveResult<'_> {
            ResolveResult::Owned(serde_json::to_value(self).unwrap())
        }

        fn get_key(&self, key: &str) -> &dyn SourceData {
            match key {
                "name" => &self.name,
                "values" => &self.values,
                _ => &crate::NULL_CONST,
            }
        }

        fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
            Box::new(["name", "values"].into_iter())
        }

        fn len(&self) -> Option<usize> {
            Some(2)
        }
    }

    #[test]
    fn test_custom_data() {
        let data = CustomData {
            name: "test".to_string(),
            values: vec![1, 2, 3],
        };
        let expr = compile_expression(
            r#"
        {
            "foo": input.name,
            "bar": input.values[1],
            "baz": input
        }
        "#,
            &["input"],
        )
        .unwrap();
        let result = expr.run_custom_input([&data as &dyn SourceData]).unwrap();
        let expected = serde_json::json!({
            "foo": "test",
            "bar": 2,
            "baz": {
                "name": "test",
                "values": [1, 2, 3]
            }
        });
        assert_eq!(result.into_owned(), expected);
    }
}
