use std::{
    collections::BTreeMap,
    fmt::{Debug, Formatter},
    sync::OnceLock,
};

use serde::Serialize;
use serde_json::Value;

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
    fn array_len(&self) -> Option<usize> {
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

    fn array_len(&self) -> Option<usize> {
        match self {
            serde_json::Value::Array(arr) => Some(arr.len()),
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

    fn array_len(&self) -> Option<usize> {
        (*self).array_len()
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

    fn array_len(&self) -> Option<usize> {
        (**self).array_len()
    }

    fn is_null(&self) -> bool {
        (**self).is_null()
    }
}

macro_rules! impl_source_data_arr {
    ($t:ty) => {
        impl<T> SourceData for $t
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

            fn array_len(&self) -> Option<usize> {
                Some(self.len())
            }
        }
    };
}

impl_source_data_arr!(Vec<T>);
impl_source_data_arr!(&[T]);

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

    fn array_len(&self) -> Option<usize> {
        match self {
            Some(v) => v.array_len(),
            None => None,
        }
    }

    fn is_null(&self) -> bool {
        self.is_none()
    }
}

macro_rules! impl_source_data_map {
    ($t:ty, $($st:tt)*) => {
        impl<T> SourceData for $t
        where
            T: SourceData,
        {
            fn resolve(&self) -> ResolveResult<'_> {
                let map: serde_json::Map<String, serde_json::Value> = self
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), v.resolve().into_owned()))
                    .collect();
                ResolveResult::Owned(serde_json::Value::Object(map))
            }

            fn get_key(&self, key: &str) -> &dyn SourceData {
                self.get(key)
                    .map(|v| v as &dyn SourceData)
                    .unwrap_or(&NULL_CONST)
            }

            fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
                Box::new(self.keys().$($st)*)
            }
        }
    };
}

// str-as-str is an unstable feature, so we need to disambiguate for the iterator.
impl_source_data_map!(std::collections::HashMap<String, T>, map(|k| k.as_str()));
impl_source_data_map!(std::collections::HashMap<&str, T>, copied());
impl_source_data_map!(BTreeMap<String, T>, map(|k| k.as_str()));
impl_source_data_map!(BTreeMap<&str, T>, copied());

impl SourceData for () {
    fn resolve(&self) -> ResolveResult<'_> {
        ResolveResult::Owned(serde_json::Value::Null)
    }

    fn is_null(&self) -> bool {
        true
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

/// A type that implements SourceData by converting the underlying data to some other
/// type that implements SourceData lazily on first access.
///
/// This is useful for cases where the conversion is expensive, and you want to avoid
/// doing it unless the data is actually accessed.
pub struct LazySourceData<T, F, R>
where
    F: Fn(&T) -> R,
{
    data: T,
    resolver: F,
    resolved: OnceLock<R>,
}

impl<T: Debug, F: Fn(&T) -> R, R> Debug for LazySourceData<T, F, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazySourceData")
            .field("data", &self.data)
            .finish()
    }
}

impl<T, F, R> Serialize for LazySourceData<T, F, R>
where
    F: Fn(&T) -> R,
    R: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.get_resolved().serialize(serializer)
    }
}

impl<T, F: Fn(&T) -> R, R> LazySourceData<T, F, R> {
    /// Create a new LazySourceData instance, with the given resolver.
    pub fn new(data: T, resolver: F) -> Self {
        Self {
            data,
            resolver,
            resolved: OnceLock::new(),
        }
    }

    /// Get the resolved data, computing it if necessary.
    fn get_resolved(&self) -> &R {
        self.resolved.get_or_init(|| (self.resolver)(&self.data))
    }
}

impl<T: Debug, F: Fn(&T) -> R, R: SourceData> SourceData for LazySourceData<T, F, R> {
    fn resolve(&self) -> ResolveResult<'_> {
        self.get_resolved().resolve()
    }

    fn array_len(&self) -> Option<usize> {
        self.get_resolved().array_len()
    }

    fn get_index(&self, index: usize) -> &dyn SourceData {
        self.get_resolved().get_index(index)
    }

    fn get_key(&self, key: &str) -> &dyn SourceData {
        self.get_resolved().get_key(key)
    }

    fn is_null(&self) -> bool {
        self.get_resolved().is_null()
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        self.get_resolved().keys()
    }
}

fn resolve_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

/// A LazySourceData that simply serializes the underlying data to JSON on first access.
pub type LazySourceDataJson<T> = LazySourceData<T, fn(&T) -> serde_json::Value, serde_json::Value>;

impl<T: Serialize> LazySourceDataJson<T> {
    /// Create a new LazySourceDataJson instance.
    pub fn new_json(value: T) -> Self {
        Self::new(value, resolve_json as fn(&T) -> serde_json::Value)
    }
}

#[cfg(test)]
mod tests {
    use kuiper_lang_macros::SourceData;
    use serde::Serialize;

    use crate::{compile_expression, expressions::source::SourceData};

    use crate as kuiper_lang;

    #[derive(Debug, Serialize, SourceData)]
    struct CustomData {
        name: String,
        values: Vec<i32>,
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

    #[test]
    fn test_lazy_source_data() {
        let data = CustomData {
            name: "test".to_string(),
            values: vec![1, 2, 3],
        };
        let lazy_data = crate::expressions::source::LazySourceDataJson::new_json(data);
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
        let result = expr
            .run_custom_input([&lazy_data as &dyn SourceData])
            .unwrap();
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

    #[derive(Debug, Serialize, SourceData)]
    struct CustomDataGeneric<'a, 'b, T, R> {
        foo: &'a str,
        bar: &'b str,
        data_1: T,
        #[source_data(rename = "data2")]
        data_2: R,
    }

    #[test]
    fn test_custom_data_generic() {
        let data = CustomDataGeneric {
            foo: "hello",
            bar: "world",
            data_1: vec![1, 2, 3],
            data_2: serde_json::json!({"a": 1, "b": 2}),
        };
        let expr = compile_expression(
            r#"
        {
            "foo": input.foo,
            "bar": input.bar,
            "data_1": input.data_1[0],
            "data_2": input.data2.b
        }"#,
            &["input"],
        )
        .unwrap();
        let result = expr.run_custom_input([&data as &dyn SourceData]).unwrap();
        let expected = serde_json::json!({
            "foo": "hello",
            "bar": "world",
            "data_1": 1,
            "data_2": 2
        });
        assert_eq!(result.into_owned(), expected);
    }
}
