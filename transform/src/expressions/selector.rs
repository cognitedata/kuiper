use std::{borrow::Cow, fmt::Display};

use serde_json::{Map, Value};

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionMeta, ExpressionType, ResolveResult},
    transform_error::TransformError,
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
    Input,
    Expression(Box<ExpressionType>),
}

#[derive(Debug, Clone)]
pub enum SelectorElement {
    Constant(String),
    Expression(Box<ExpressionType>),
}

impl Display for SelectorElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorElement::Constant(x) => write!(f, "{x}"),
            SelectorElement::Expression(x) => write!(f, "[{x}]"),
        }
    }
}

impl Display for SourceElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceElement::Input => write!(f, "$"),
            SourceElement::Expression(e) => write!(f, "{e}"),
        }
    }
}

impl Display for SelectorExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)?;
        let mut initial = true;
        for el in &self.path {
            if matches!(el, SelectorElement::Constant(_)) && !initial {
                write!(f, ".")?;
            }
            initial = false;
            write!(f, "{el}")?;
        }
        Ok(())
    }
}

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for SelectorExpression {
    fn resolve(
        &'a self,
        state: &'b ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        match &self.source {
            SourceElement::Input => {
                let first_sel = self.path.first().unwrap();
                let source_ref = match first_sel {
                    SelectorElement::Constant(x) => match state.get_value(x) {
                        Some(x) => x,
                        None => {
                            return Err(TransformError::new_source_missing(
                                x.to_string(),
                                &self.span,
                                state.id,
                            ))
                        }
                    },
                    SelectorElement::Expression(x) => {
                        let val = x.resolve(state)?;
                        match val.as_ref() {
                            Value::String(s) => match state.get_value(s) {
                                Some(x) => x,
                                None => {
                                    return Err(TransformError::new_source_missing(
                                        s.to_string(),
                                        &self.span,
                                        state.id,
                                    ))
                                }
                            },
                            Value::Number(_) => {
                                return Err(TransformError::new_invalid_operation(
                                    "Root selector must be string".to_string(),
                                    &self.span,
                                    state.id,
                                ))
                            }
                            _ => {
                                return Err(TransformError::new_incorrect_type(
                                    "Incorrect type in selector",
                                    "string",
                                    TransformError::value_desc(&val),
                                    &self.span,
                                    state.id,
                                ))
                            }
                        }
                    }
                };
                self.resolve_by_reference(source_ref, state, true)
            }
            SourceElement::Expression(e) => {
                let src = e.resolve(state)?;
                match src {
                    Cow::Borrowed(v) => self.resolve_by_reference(v, state, false),
                    Cow::Owned(v) => self.resolve_by_value(v, state),
                }
            }
        }
    }
}

impl<'a: 'c, 'b, 'c> ExpressionMeta<'a, 'b, 'c> for SelectorExpression {
    fn num_children(&self) -> usize {
        0
    }

    fn get_child(&self, _idx: usize) -> Option<&ExpressionType> {
        None
    }

    fn get_child_mut(&mut self, _idx: usize) -> Option<&mut ExpressionType> {
        None
    }

    fn set_child(&mut self, _idx: usize, _item: ExpressionType) {}
}

impl SelectorExpression {
    pub fn new(source: SourceElement, path: Vec<SelectorElement>, span: Span) -> Self {
        Self { source, path, span }
    }

    fn resolve_by_reference<'a: 'c, 'b, 'c>(
        &'a self,
        source: &'c Value,
        state: &'b ExpressionExecutionState<'c, 'b>,
        skip: bool,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let mut elem = source;
        for p in self.path.iter().skip(if skip { 1 } else { 0 }) {
            elem = match p {
                SelectorElement::Constant(x) => match elem.as_object().and_then(|o| o.get(x)) {
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
                        Value::Number(n) => match n.as_u64() {
                            Some(x) => match elem.as_array().and_then(|a| a.get(x as usize)) {
                                Some(x) => x,
                                None => return Ok(ResolveResult::Owned(Value::Null)),
                            },
                            _ => {
                                return Err(TransformError::new_incorrect_type(
                                    "Incorrect type in selector",
                                    "positive integer",
                                    if n.is_f64() {
                                        "floating point"
                                    } else {
                                        "negative integer"
                                    },
                                    &self.span,
                                    state.id,
                                ))
                            }
                        },
                        _ => {
                            return Err(TransformError::new_incorrect_type(
                                "Incorrect type in selector",
                                "integer or string",
                                TransformError::value_desc(&val),
                                &self.span,
                                state.id,
                            ))
                        }
                    }
                }
            };
        }
        Ok(ResolveResult::Borrowed(elem))
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

    fn resolve_by_value(
        &self,
        source: Value,
        state: &ExpressionExecutionState<'_, '_>,
    ) -> Result<ResolveResult<'_>, TransformError> {
        let mut elem = source;
        for p in self.path.iter() {
            elem = match p {
                SelectorElement::Constant(x) => {
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
                        Value::Number(n) => match n.as_u64() {
                            Some(x) => {
                                match Self::as_array_owned(elem)
                                    .and_then(|a| a.into_iter().nth(x as usize))
                                {
                                    Some(x) => x,
                                    None => return Ok(ResolveResult::Owned(Value::Null)),
                                }
                            }
                            _ => {
                                return Err(TransformError::new_incorrect_type(
                                    "Incorrect type in selector",
                                    "positive integer",
                                    if n.is_f64() {
                                        "floating point"
                                    } else {
                                        "negative integer"
                                    },
                                    &self.span,
                                    state.id,
                                ))
                            }
                        },
                        _ => {
                            return Err(TransformError::new_incorrect_type(
                                "Incorrect type in selector",
                                "integer or string",
                                TransformError::value_desc(&val),
                                &self.span,
                                state.id,
                            ))
                        }
                    }
                }
            };
        }
        Ok(ResolveResult::Owned(elem))
    }
}
