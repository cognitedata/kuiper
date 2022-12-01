use std::fmt::Display;

use serde_json::Value;

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionType, ResolveResult},
    transform_error::TransformError,
};
use logos::Span;

/// Selector expression, used to get a field from an input.
pub struct SelectorExpression {
    source: SelectorElement,
    path: Vec<SelectorElement>,
    span: Span,
}

pub enum SelectorElement {
    Constant(String),
    Expression(Box<ExpressionType>),
}

impl Display for SelectorElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorElement::Constant(x) => write!(f, "{}", x),
            SelectorElement::Expression(x) => write!(f, "[{}]", x),
        }
    }
}

impl Display for SelectorExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}", self.source)?;
        for el in &self.path {
            write!(f, ".{}", el)?;
        }
        Ok(())
    }
}

impl<'a> Expression<'a> for SelectorExpression {
    fn resolve(
        &'a self,
        state: &'a ExpressionExecutionState,
    ) -> Result<ResolveResult<'a>, TransformError> {
        let source = match &self.source {
            SelectorElement::Constant(x) => match state.get_value(x) {
                Some(x) => x,
                None => {
                    return Err(TransformError::new_source_missing(
                        self.source.to_string(),
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
                                self.source.to_string(),
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
                            TransformError::value_desc(val.as_ref()),
                            &self.span,
                            state.id,
                        ))
                    }
                }
            }
        };

        let mut elem = source;
        for p in &self.path {
            elem = match p {
                SelectorElement::Constant(x) => match elem.as_object().and_then(|o| o.get(x)) {
                    Some(x) => x,
                    None => return Ok(ResolveResult::Value(Value::Null)),
                },
                SelectorElement::Expression(x) => {
                    let val = x.resolve(state)?;
                    match val.as_ref() {
                        Value::String(s) => match elem.as_object().and_then(|o| o.get(s)) {
                            Some(x) => x,
                            None => return Ok(ResolveResult::Value(Value::Null)),
                        },
                        Value::Number(n) => match n.as_u64() {
                            Some(x) => match elem.as_array().and_then(|a| a.get(x as usize)) {
                                Some(x) => x,
                                None => return Ok(ResolveResult::Value(Value::Null)),
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
                                TransformError::value_desc(val.as_ref()),
                                &self.span,
                                state.id,
                            ))
                        }
                    }
                }
            };
        }
        Ok(ResolveResult::Reference(elem))
    }
}

impl SelectorExpression {
    pub fn new(source: SelectorElement, path: Vec<SelectorElement>, span: Span) -> Self {
        Self { source, path, span }
    }
}
