use std::fmt::Display;

use serde_json::{Map, Value};

use crate::{compiler::BuildError, NULL_CONST};

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionMeta, ExpressionType},
    numbers::JsonNumber,
    transform_error::TransformError,
    ResolveResult,
};

use logos::Span;
#[derive(Debug, Clone)]
/// Selector expression, used to get a field from an input.
pub struct SelectorExpression {
    source: SourceElement,
    path: Vec<SelectorElement>,
    span: Span,
}

#[derive(Debug, Clone)]
pub enum SourceElement {
    CompiledInput(usize),
    Expression(Box<ExpressionType>),
}

#[derive(Debug, Clone)]
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

impl<'a: 'c, 'c> Expression<'a, 'c> for SelectorExpression {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, TransformError> {
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
                self.resolve_by_reference(source_ref, state)
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
                                        elem.as_array().and_then(|a| a.get(a.len() - (-n as usize)))
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
            state.add_completion_entries(o.keys(), s.clone());
        }
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
        }
        Ok(ResolveResult::Owned(elem))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::compile_expression;

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
    }
}
