use std::fmt::Display;

use serde_json::Value;

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionType, ResolveResult},
    transform_error::TransformError,
};

pub struct SelectorExpression {
    source: SelectorElement,
    path: Vec<SelectorElement>,
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
            SelectorElement::Constant(x) => match state.data.get(x) {
                Some(x) => *x,
                None => return Err(TransformError::SourceMissingError(self.source.to_string())),
            },
            SelectorElement::Expression(x) => {
                let val = x.resolve(state)?;
                match val.as_ref() {
                    Value::String(s) => match state.data.get(s) {
                        Some(x) => x,
                        None => {
                            return Err(TransformError::SourceMissingError(self.source.to_string()))
                        }
                    },
                    Value::Number(n) => match n.as_f64() {
                        _ => {
                            return Err(TransformError::SourceMissingError(
                                "Root selector must be string".to_string(),
                            ))
                        }
                    },
                    _ => {
                        return Err(TransformError::new_incorrect_type(
                            "selector",
                            "integer or string",
                            &val.as_ref(),
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
                            None => return Ok(ResolveResult::Value(Value::Null))
                        },
                        Value::Number(n) => match n.as_u64() {
                            Some(x) => match elem.as_array().and_then(|a| a.get(x as usize)) {
                                Some(x) => x,
                                None => return Ok(ResolveResult::Value(Value::Null))
                            },
                            _ => return Err(TransformError::IncorrectTypeInField("Incorrect type in selector. Expected positive integer, got floating point".to_string())),
                        },
                        _ => {
                            return Err(TransformError::new_incorrect_type(
                                "selector",
                                "integer or string",
                                &val.as_ref(),
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
    pub fn new(source: SelectorElement, path: Vec<SelectorElement>) -> Self {
        Self { source, path }
    }
}
