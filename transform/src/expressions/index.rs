use std::{borrow::Cow, fmt::Display};

use logos::Span;
use serde_json::Value;

use crate::TransformError;

use super::{
    base::{get_number_from_value, get_string_from_value},
    Expression, ExpressionType, ResolveResult,
};

#[derive(Debug, Clone)]
pub enum SelectorElement {
    Constant(String),
    Expression(Box<ExpressionType>),
}

pub enum SelectorSource {
    Expression(Box<ExpressionType>),
    Input,
}

pub struct IndexExpression {
    selector: SelectorElement,
    source: SelectorSource,
    span: Span,
}

impl Display for SelectorElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorElement::Constant(x) => write!(f, "{x}"),
            SelectorElement::Expression(x) => write!(f, "[{x}]"),
        }
    }
}

impl Display for SelectorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorSource::Expression(e) => write!(f, "{e}"),
            SelectorSource::Input => write!(f, "$"),
        }
    }
}

impl Display for IndexExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)?;
        if matches!(self.source, SelectorSource::Input) {
            write!(f, "{}", self.selector)
        } else {
            write!(f, ".{}", self.selector)
        }
    }
}

impl<'a: 'c, 'b, 'c> Expression<'a, 'b, 'c> for IndexExpression {
    fn resolve(
        &'a self,
        state: &'b super::ExpressionExecutionState<'c, 'b>,
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        match &self.source {
            SelectorSource::Expression(e) => {
                let source_res = e.resolve(state)?;
                match source_res.as_ref() {
                    serde_json::Value::Array(a) => match &self.selector {
                        SelectorElement::Constant(_) => {
                            return Err(TransformError::new_invalid_operation(
                                "Attempted to index into array by string".to_string(),
                                &self.span,
                                state.id,
                            ))
                        }
                        SelectorElement::Expression(e) => {
                            let e_res = e.resolve(state)?;
                            let idx_num = get_number_from_value(
                                "index",
                                e_res.as_ref(),
                                &self.span,
                                state.id,
                            )?;
                            let idx = idx_num.try_as_u64(&self.span, state.id)?;
                            match a.get(idx as usize) {
                                Some(x) => Ok(ResolveResult::Borrowed(x)),
                                None => Ok(ResolveResult::Owned(Value::Null)),
                            }
                        }
                    },
                    serde_json::Value::Object(o) => {
                        let idx_str = match &self.selector {
                            SelectorElement::Constant(x) => Cow::Borrowed(x),
                            SelectorElement::Expression(e) => {
                                let e_res = e.resolve(state)?;
                                get_string_from_value(
                                    "index",
                                    e_res.as_ref(),
                                    &self.span,
                                    state.id,
                                )?
                            }
                        };
                        match o.get(idx_str.as_ref()) {
                            Some(x) => Ok(ResolveResult::Borrowed(x)),
                            None => Ok(ResolveResult::Owned(Value::Null)),
                        }
                    }
                    r => {
                        return Err(TransformError::new_incorrect_type(
                            "index",
                            "object",
                            TransformError::value_desc(&r),
                            &self.span,
                            state.id,
                        ))
                    }
                }
            }
            SelectorSource::Input => todo!(),
        }
    }
}
