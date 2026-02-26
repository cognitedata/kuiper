use std::fmt::Display;

use serde_json::{Map, Value};

use crate::{
    compiler::BuildError,
    expressions::source::SourceData,
    types::{Type, TypeError},
    NULL_CONST,
};

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionMeta, ExpressionType},
    numbers::JsonNumber,
    transform_error::TransformError,
    ResolveResult,
};

use logos::Span;
#[derive(Debug)]
/// Selector expression, used to get a field from an input.
pub struct SelectorExpression {
    source: SourceElement,
    path: Vec<SelectorElement>,
    span: Span,
}

#[derive(Debug)]
pub enum SourceElement {
    CompiledInput(usize),
    Expression(Box<ExpressionType>),
}

#[derive(Debug)]
pub enum SelectorElement {
    Constant(String, Span),
    Expression(Box<ExpressionType>),
}

impl Display for SelectorElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorElement::Constant(x, _) => write!(f, "{x}"),
            SelectorElement::Expression(x) => write!(f, "[{x}]"),
        }
    }
}

impl Display for SourceElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceElement::CompiledInput(s) => write!(f, "${s}"),
            SourceElement::Expression(e) => write!(f, "{e}"),
        }
    }
}

impl Display for SelectorExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)?;
        for el in &self.path {
            if matches!(el, SelectorElement::Constant(_, _)) {
                write!(f, ".")?;
            }
            write!(f, "{el}")?;
        }
        Ok(())
    }
}

impl Expression for SelectorExpression {
    fn resolve<'a>(
        &'a self,
        state: &mut ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        match &self.source {
            SourceElement::CompiledInput(i) => {
                let source_ref = match state.get_value(*i) {
                    Some(x) => x,
                    None => {
                        return Err(TransformError::new_source_missing(
                            i.to_string(),
                            &self.span,
                        ))
                    }
                };
                self.resolve_source_reference(source_ref, state)
            }
            SourceElement::Expression(e) => {
                let src = e.resolve(state)?;
                match src {
                    ResolveResult::Borrowed(v) => self.resolve_by_reference(v, state),
                    ResolveResult::Owned(v) => self.resolve_by_value(v, state),
                }
            }
        }
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let ty = match &self.source {
            SourceElement::CompiledInput(i) => state.get_type(*i).cloned().unwrap_or(Type::null()),
            SourceElement::Expression(e) => e.resolve_types(state)?,
        };

        let mut elem = ty;
        for p in &self.path {
            if matches!(elem, Type::Any) {
                return Ok(Type::Any);
            }
            elem = match p {
                SelectorElement::Constant(x, _) => {
                    let Ok(obj_ty) = elem.try_as_object(&self.span) else {
                        return Ok(Type::null());
                    };
                    let Some(inner) = obj_ty.index_into(x.as_str()) else {
                        return Ok(Type::null());
                    };
                    inner
                }
                SelectorElement::Expression(e) => {
                    let val = e.resolve_types(state)?;
                    Self::resolve_type_field(val, &elem, &self.span)?
                }
            };
        }
        Ok(elem)
    }
}

impl ExpressionMeta for SelectorExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        let iter = self.path.iter_mut().filter_map(|f| match f {
            SelectorElement::Expression(e) => Some(e.as_mut()),
            _ => None,
        });
        match &mut self.source {
            SourceElement::Expression(e) => Box::new(iter.chain([e.as_mut()])),
            _ => Box::new(iter),
        }
    }
}

impl SelectorExpression {
    pub fn new(
        source: SourceElement,
        path: Vec<SelectorElement>,
        span: Span,
    ) -> Result<Self, BuildError> {
        if let SourceElement::Expression(expr) = &source {
            expr.fail_if_lambda()?;
        }
        for item in &path {
            if let SelectorElement::Expression(expr) = &item {
                expr.fail_if_lambda()?;
            }
        }
        Ok(Self { source, path, span })
    }

    fn resolve_source_reference<'a: 'c, 'c>(
        &'a self,
        source: &'c dyn SourceData,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let mut elem = source;
        state.inc_op()?;

        for p in self.path.iter() {
            state.inc_op()?;

            #[cfg(feature = "completions")]
            Self::register_completions_source(state, p, elem);

            elem = match p {
                SelectorElement::Constant(x, _) => elem.get_key(x),
                SelectorElement::Expression(x) => {
                    let val = x.resolve(state)?;
                    match val.as_ref() {
                        Value::String(s) => elem.get_key(s),
                        Value::Number(n) => {
                            let num = JsonNumber::from(n.clone());
                            match num {
                                JsonNumber::PosInteger(n) => elem.get_index(n as usize),
                                JsonNumber::NegInteger(n) => {
                                    if n < 0 {
                                        elem.array_len()
                                            .unwrap_or(0)
                                            .checked_sub(-n as usize)
                                            .map_or(&NULL_CONST, |i| elem.get_index(i))
                                    } else {
                                        elem.get_index(n as usize)
                                    }
                                }
                                _ => {
                                    return Err(TransformError::new_incorrect_type(
                                        "Incorrect type in selector",
                                        "integer",
                                        "floating point",
                                        &self.span,
                                    ))
                                }
                            }
                        }
                        _ => {
                            return Err(TransformError::new_incorrect_type(
                                "Incorrect type in selector",
                                "integer or string",
                                TransformError::value_desc(&val),
                                &self.span,
                            ))
                        }
                    }
                }
            };
            if elem.is_null() {
                break;
            }
        }
        Ok(elem.resolve())
    }

    fn resolve_by_reference<'a: 'c, 'c>(
        &'a self,
        source: &'c Value,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let mut elem = source;
        state.inc_op()?;
        for p in self.path.iter() {
            state.inc_op()?;

            #[cfg(feature = "completions")]
            Self::register_completions(state, p, elem);

            elem = match p {
                SelectorElement::Constant(x, _) => match elem.as_object().and_then(|o| o.get(x)) {
                    Some(x) => x,
                    None => return Ok(ResolveResult::Owned(Value::Null)),
                },
                SelectorElement::Expression(x) => {
                    let val = x.resolve(state)?;
                    match val.as_ref() {
                        Value::String(s) => match elem.as_object().and_then(|o| o.get(s)) {
                            Some(x) => x,
                            None => return Ok(ResolveResult::Owned(Value::Null)),
                        },
                        Value::Number(n) => {
                            let num = JsonNumber::from(n.clone());
                            let val = match num {
                                JsonNumber::PosInteger(n) => {
                                    elem.as_array().and_then(|a| a.get(n as usize))
                                }
                                JsonNumber::NegInteger(n) => {
                                    if n < 0 {
                                        elem.as_array().and_then(|a| {
                                            a.len().checked_sub(-n as usize).and_then(|l| a.get(l))
                                        })
                                    } else {
                                        elem.as_array().and_then(|a| a.get(n as usize))
                                    }
                                }
                                _ => {
                                    return Err(TransformError::new_incorrect_type(
                                        "Incorrect type in selector",
                                        "integer",
                                        "floating point",
                                        &self.span,
                                    ))
                                }
                            };
                            val.unwrap_or(&NULL_CONST)
                        }
                        _ => {
                            return Err(TransformError::new_incorrect_type(
                                "Incorrect type in selector",
                                "integer or string",
                                TransformError::value_desc(&val),
                                &self.span,
                            ))
                        }
                    }
                }
            };
            if elem.is_null() {
                break;
            }
        }
        Ok(ResolveResult::Borrowed(elem))
    }

    #[cfg(feature = "completions")]
    fn register_completions(
        state: &mut ExpressionExecutionState<'_, '_>,
        p: &SelectorElement,
        source: &Value,
    ) {
        let SelectorElement::Constant(_, s) = p else {
            return;
        };
        if let Some(o) = source.as_object() {
            state.add_completion_entries(|| o.keys(), s.clone());
        }
    }

    #[cfg(feature = "completions")]
    fn register_completions_source(
        state: &mut ExpressionExecutionState<'_, '_>,
        p: &SelectorElement,
        source: &dyn SourceData,
    ) {
        let SelectorElement::Constant(_, s) = p else {
            return;
        };
        state.add_completion_entries(|| source.keys(), s.clone());
    }

    fn as_object_owned(value: Value) -> Option<Map<String, Value>> {
        match value {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    fn as_array_owned(value: Value) -> Option<Vec<Value>> {
        match value {
            Value::Array(o) => Some(o),
            _ => None,
        }
    }

    fn resolve_by_value<'a: 'b, 'b>(
        &'a self,
        source: Value,
        state: &mut ExpressionExecutionState<'b, '_>,
    ) -> Result<ResolveResult<'b>, TransformError> {
        let mut elem = source;
        state.inc_op()?;
        for p in self.path.iter() {
            state.inc_op()?;

            #[cfg(feature = "completions")]
            Self::register_completions(state, p, &elem);

            elem = match p {
                SelectorElement::Constant(x, _) => {
                    match Self::as_object_owned(elem).and_then(|mut o| o.remove(x)) {
                        Some(x) => x,
                        None => return Ok(ResolveResult::Owned(Value::Null)),
                    }
                }
                SelectorElement::Expression(x) => {
                    let val = x.resolve(state)?;
                    match val.as_ref() {
                        Value::String(s) => {
                            match Self::as_object_owned(elem).and_then(|mut o| o.remove(s)) {
                                Some(x) => x,
                                None => return Ok(ResolveResult::Owned(Value::Null)),
                            }
                        }
                        Value::Number(n) => {
                            let num = JsonNumber::from(n.clone());
                            let val = match num {
                                JsonNumber::PosInteger(n) => Self::as_array_owned(elem)
                                    .and_then(|a| a.into_iter().nth(n as usize)),
                                JsonNumber::NegInteger(n) => {
                                    if n < 0 {
                                        Self::as_array_owned(elem).and_then(|a| {
                                            a.into_iter().rev().nth((-n - 1) as usize)
                                        })
                                    } else {
                                        Self::as_array_owned(elem)
                                            .and_then(|a| a.into_iter().nth(n as usize))
                                    }
                                }
                                _ => {
                                    return Err(TransformError::new_incorrect_type(
                                        "Incorrect type in selector",
                                        "integer",
                                        "floating point",
                                        &self.span,
                                    ))
                                }
                            };
                            val.unwrap_or(Value::Null)
                        }
                        _ => {
                            return Err(TransformError::new_incorrect_type(
                                "Incorrect type in selector",
                                "integer or string",
                                TransformError::value_desc(&val),
                                &self.span,
                            ))
                        }
                    }
                }
            };
            if elem.is_null() {
                break;
            }
        }
        Ok(ResolveResult::Owned(elem))
    }

    fn resolve_type_field(
        selector: Type,
        select_from: &Type,
        span: &Span,
    ) -> Result<Type, TypeError> {
        Ok(match selector {
            Type::Constant(Value::String(s)) => {
                let Ok(obj_ty) = select_from.try_as_object(span) else {
                    return Ok(Type::null());
                };
                let Some(inner) = obj_ty.index_into(s.as_str()) else {
                    return Ok(Type::null());
                };
                inner
            }
            Type::Constant(Value::Number(n)) => {
                let Ok(arr_ty) = select_from.try_as_array(span) else {
                    return Ok(Type::null());
                };

                let num = JsonNumber::from(n.clone());
                match num {
                    JsonNumber::PosInteger(n) => {
                        arr_ty.index_into(n as usize).unwrap_or_else(Type::null)
                    }
                    JsonNumber::NegInteger(n) => {
                        if n < 0 {
                            arr_ty
                                .index_from_end((-n - 1) as usize)
                                .unwrap_or_else(Type::null)
                        } else {
                            arr_ty.index_into(n as usize).unwrap_or_else(Type::null)
                        }
                    }
                    _ => {
                        return Err(TypeError::expected_type(
                            Type::Integer,
                            Type::Float,
                            span.clone(),
                        ))
                    }
                }
            }
            Type::Any => match &select_from {
                Type::Object(o) => o.element_union().union_with(Type::null()),
                Type::Array(s) => s.element_union().union_with(Type::null()),
                Type::Union(u) => {
                    let mut typ = Type::null();
                    for t in u {
                        match t {
                            Type::Object(o) => {
                                typ = typ.union_with(o.element_union());
                            }
                            Type::Array(s) => {
                                typ = typ.union_with(s.element_union());
                            }
                            _ => (),
                        }
                    }
                    typ
                }
                Type::Any => Type::Any,
                _ => Type::null(),
            },
            Type::Union(u) => {
                let mut typ = Type::null();
                for t in &u {
                    if let Ok(res) = Self::resolve_type_field(t.clone(), select_from, span) {
                        typ = typ.union_with(res);
                    }
                }
                if typ.is_never() {
                    return Err(TypeError::expected_type(
                        Type::Union(vec![Type::String, Type::Integer]),
                        Type::Union(u),
                        span.clone(),
                    ));
                }
                typ
            }
            Type::Integer => {
                let Ok(arr_ty) = select_from.try_as_array(span) else {
                    return Ok(Type::null());
                };
                arr_ty.element_union().union_with(Type::null())
            }
            Type::String => {
                let Ok(obj_ty) = select_from.try_as_object(span) else {
                    return Ok(Type::null());
                };
                obj_ty.element_union().union_with(Type::null())
            }
            _ => {
                return Err(TypeError::expected_type(
                    Type::Union(vec![Type::String, Type::Integer]),
                    selector,
                    span.clone(),
                ))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{
        compile_expression,
        types::{Array, Object, Type},
    };

    #[test]
    fn test_constant_selector() {
        let expr = compile_expression(
            r#"{
                "1": { "foo": 123 }.foo,
                "2": { "foo": { "bar": 456 } }.foo.bar,
                "3": { "foo": 123 }.baz,
                "4": "hello".test,
                "5": null.test,
                "6": { "foo": 123 }.foo.bar,
            }
            "#,
            &[],
        )
        .unwrap();
        let r = expr.run(&[]).unwrap().into_owned();
        assert_eq!(r.get("1").unwrap(), &Value::from(123));
        assert_eq!(r.get("2").unwrap(), &Value::from(456));
        assert_eq!(r.get("3").unwrap(), &Value::Null);
        assert_eq!(r.get("4").unwrap(), &Value::Null);
        assert_eq!(r.get("5").unwrap(), &Value::Null);
        assert_eq!(r.get("6").unwrap(), &Value::Null);
    }

    #[test]
    fn test_dynamic_selector() {
        let expr = compile_expression(
            r#"{
            "1": { "foo": 123 }["foo"],
            "2": { "bar": { "foo": 456 } }["bar"]["foo"],
            "3": { "foo": 123 }["baz"],
            "4": "hello"["test"],
            "5": null["test"],
            "6": { "foo": 123 }["foo"]["bar"],
        }"#,
            &[],
        )
        .unwrap();
        let r = expr.run(&[]).unwrap().into_owned();
        assert_eq!(r.get("1").unwrap(), &Value::from(123));
        assert_eq!(r.get("2").unwrap(), &Value::from(456));
        assert_eq!(r.get("3").unwrap(), &Value::Null);
        assert_eq!(r.get("4").unwrap(), &Value::Null);
        assert_eq!(r.get("5").unwrap(), &Value::Null);
        assert_eq!(r.get("6").unwrap(), &Value::Null);
    }

    #[test]
    fn test_array_selector() {
        let expr = compile_expression(
            r#"{
            "1": [1, 2, 3][1],
            "2": [1, 2, 3][-1],
            "3": [1, 2, [1, 2]][2][1],
            "4": [1, 2, 3][3],
            "5": [1, 2, 3][0],
            "6": [1, 2, 3][-5],
            "7": { "foo": 123 }[0],
            "8": null[1][2],
            "9": [1, 2, 3][3][3],
            "10": [1, 2, 3][-4],
        }"#,
            &[],
        )
        .unwrap();
        let r = expr.run(&[]).unwrap().into_owned();
        assert_eq!(r.get("1").unwrap(), &Value::from(2));
        assert_eq!(r.get("2").unwrap(), &Value::from(3));
        assert_eq!(r.get("3").unwrap(), &Value::from(2));
        assert_eq!(r.get("4").unwrap(), &Value::Null);
        assert_eq!(r.get("5").unwrap(), &Value::from(1));
        assert_eq!(r.get("6").unwrap(), &Value::Null);
        assert_eq!(r.get("7").unwrap(), &Value::Null);
        assert_eq!(r.get("8").unwrap(), &Value::Null);
        assert_eq!(r.get("9").unwrap(), &Value::Null);
        assert_eq!(r.get("10").unwrap(), &Value::Null);
    }

    #[test]
    fn test_selector_types_array() {
        let expr = crate::compile_expression(
            r#"
        input[0]
        "#,
            &["input"],
        )
        .unwrap();
        let r = expr
            .run_types([Type::array_of_type(Type::Boolean)])
            .unwrap();
        // The array might be empty, so result could be null.
        assert_eq!(r, Type::Boolean.union_with(Type::null()));
        let r = expr
            .run_types([Type::Array(Array {
                elements: vec![Type::Integer, Type::Boolean],
                end_dynamic: Some(Box::new(Type::Float)),
            })])
            .unwrap();
        assert_eq!(r, Type::Integer);

        // Empty array
        let r = expr.run_types([Type::Array(Array::default())]).unwrap();
        assert_eq!(r, Type::null());
    }

    #[test]
    fn test_selector_types_array_neg() {
        let expr = crate::compile_expression(
            r#"
        input[-1]
        "#,
            &["input"],
        )
        .unwrap();
        let r = expr
            .run_types([Type::array_of_type(Type::Boolean)])
            .unwrap();
        // The array might be empty, so result could be null.
        assert_eq!(r, Type::Boolean.union_with(Type::null()));

        let r = expr
            .run_types([Type::Array(Array {
                elements: vec![Type::Integer, Type::Boolean],
                end_dynamic: Some(Box::new(Type::Float)),
            })])
            .unwrap();
        // The result could either be the last element in the known static array, or
        // the end_dynamic type. It can't be the first type, since that would require us to take
        // at least 2 elements from the array.
        assert_eq!(r, Type::Float.union_with(Type::Boolean));

        // Empty array
        let r = expr.run_types([Type::Array(Array::default())]).unwrap();
        assert_eq!(r, Type::null());
    }

    #[test]
    fn test_selector_types_array_dynamic() {
        let expr = crate::compile_expression(
            r#"
            input[now()]
            "#,
            &["input"],
        )
        .unwrap();
        let r = expr
            .run_types([Type::Array(Array {
                elements: vec![Type::Integer, Type::Boolean],
                end_dynamic: Some(Box::new(Type::Float)),
            })])
            .unwrap();
        // We have no idea what the selector is, the result is any element in the array,
        // or outside the array (null).
        assert_eq!(
            r,
            Type::Integer
                .union_with(Type::Boolean)
                .union_with(Type::Float)
                .union_with(Type::null())
        );
    }

    #[test]
    fn test_selector_types_object() {
        let expr = crate::compile_expression(
            r#"
            input.foo
            "#,
            &["input"],
        )
        .unwrap();
        // Just all dynamic.
        let r = expr
            .run_types([Type::object_of_type(Type::Boolean)])
            .unwrap();
        // foo might not be in the object at all.
        assert_eq!(r, Type::Boolean.nullable());

        // Foo is defined.
        let r = expr
            .run_types([Type::Object(
                Object::default()
                    .with_field("foo", Type::Float)
                    .with_field("bar", Type::Integer),
            )])
            .unwrap();
        assert_eq!(r, Type::Float);

        // Foo isn't defined.
        let r = expr
            .run_types([Type::Object(
                Object::default()
                    .with_field("bar", Type::Float)
                    .with_field("baz", Type::Integer),
            )])
            .unwrap();
        assert_eq!(r, Type::null());
    }

    #[test]
    fn test_selector_types_object_const_dynamic() {
        let expr = crate::compile_expression(
            r#"
            input["foo"]
            "#,
            &["input"],
        )
        .unwrap();
        // Should yield the same as just `input.foo`.
        // Just all dynamic.
        let r = expr
            .run_types([Type::object_of_type(Type::Boolean)])
            .unwrap();
        // foo might not be in the object at all.
        assert_eq!(r, Type::Boolean.nullable());

        // Foo is defined.
        let r = expr
            .run_types([Type::Object(
                Object::default()
                    .with_field("foo", Type::Float)
                    .with_field("bar", Type::Integer),
            )])
            .unwrap();
        assert_eq!(r, Type::Float);
    }

    #[test]
    fn test_selector_types_object_dynamic() {
        let expr = crate::compile_expression(
            r#"
            input[string(now())]"#,
            &["input"],
        )
        .unwrap();
        // We don't know what the key is, so we just return the union of every field.
        let r = expr
            .run_types([Type::Object(
                Object::default()
                    .with_field("foo", Type::Float)
                    .with_field("bar", Type::Integer)
                    .with_field("baz", Type::from_const(123))
                    .with_generic_field(Type::Boolean),
            )])
            .unwrap();
        // The constant 123 is eaten by the generic integer, since {123} ⊂ Integer
        assert_eq!(
            r,
            Type::Integer
                .union_with(Type::Float)
                .union_with(Type::Boolean)
                .union_with(Type::null())
        );
    }
}
