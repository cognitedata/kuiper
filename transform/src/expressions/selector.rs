use std::fmt::Display;

use serde_json::Value;

use super::{
    base::{Expression, ExpressionExecutionState, ExpressionType},
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

enum PathElement {
    Integer(u64),
    String(String),
}

impl Display for SelectorElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorElement::Constant(x) => write!(f, "{}", x),
            SelectorElement::Expression(x) => write!(f, "[{}]", x),
        }
    }
}

impl SelectorElement {
    fn evaluate(&self, state: &ExpressionExecutionState) -> Result<PathElement, TransformError> {
        match self {
            Self::Constant(x) => Ok(PathElement::String(x.clone())),
            Self::Expression(x) => {
                let val = x.resolve(state)?;
                match val {
                    Value::String(s) => Ok(PathElement::String(s)),
                    Value::Number(n) => match n.as_f64() {
                        Some(x) if x.fract() == 0.0 && x >= u64::MIN as f64 && x <= u64::MAX as f64 => Ok(PathElement::Integer(x as u64)),
                        _ => Err(TransformError::IncorrectTypeInField("Incorrect type in selector. Expected positive integer, got floating point".to_string())),
                    }
                    _ => Err(TransformError::new_incorrect_type("selector", "integer or string", &val))
                }
            }
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

impl Expression for SelectorExpression {
    fn resolve(&self, state: &ExpressionExecutionState) -> Result<Value, TransformError> {
        let source_key = self.source.evaluate(state)?;
        let source_key = match source_key {
            PathElement::String(s) => s,
            PathElement::Integer(_) => {
                return Err(TransformError::IncorrectTypeInField(
                    "Incorrect type in selector, first element must be a string".to_string(),
                ))
            }
        };
        let source = state.data.get(&source_key);
        let source =
            source.ok_or_else(|| TransformError::SourceMissingError(self.source.to_string()))?;

        let mut elem = source;
        for p in &self.path {
            elem = match p.evaluate(state)? {
                PathElement::Integer(i) => match elem.as_array().and_then(|a| a.get(i as usize)) {
                    Some(x) => x,
                    None => return Ok(Value::Null),
                },
                PathElement::String(s) => match elem.as_object().and_then(|o| o.get(&s)) {
                    Some(x) => x,
                    None => return Ok(Value::Null),
                },
            };
        }
        Ok(elem.clone())
    }
}

impl SelectorExpression {
    pub fn new(source: SelectorElement, path: Vec<SelectorElement>) -> Self {
        Self { source, path }
    }
}
