use std::{borrow::Cow, fmt::Display};

use serde_json::{Map, Value};

use crate::compiler::BuildError;

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
        state: &ExpressionExecutionState<'c, '_>,
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
                    Cow::Borrowed(v) => self.resolve_by_reference(v, state),
                    Cow::Owned(v) => self.resolve_by_value(v, state),
                }
            }
        }
    }
}

impl ExpressionMeta for SelectorExpression {
    fn num_children(&self) -> usize {
        let path = self.path.iter().filter_map(|f| match f {
            SelectorElement::Expression(e) => Some(e.as_ref()),
            _ => None,
        });
        match &self.source {
            SourceElement::Expression(e) => [e.as_ref()].into_iter().chain(path).count(),
            _ => path.count(),
        }
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        let mut path = self.path.iter().filter_map(|f| match f {
            SelectorElement::Expression(e) => Some(e.as_ref()),
            _ => None,
        });
        match &self.source {
            SourceElement::Expression(e) => [e.as_ref()].into_iter().chain(path).nth(idx),
            _ => path.nth(idx),
        }
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        let mut path = self.path.iter_mut().filter_map(|f| match f {
            SelectorElement::Expression(e) => Some(e.as_mut()),
            _ => None,
        });
        match &mut self.source {
            SourceElement::Expression(e) => [e.as_mut()].into_iter().chain(path).nth(idx),
            _ => path.nth(idx),
        }
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        let add = if matches!(self.source, SourceElement::Expression(_)) {
            1
        } else {
            0
        };
        let mut path = self.path.iter().enumerate().filter_map(|(idx, f)| match f {
            SelectorElement::Expression(e) => Some((idx + add, e.as_ref())),
            _ => None,
        });
        let real_idx = match &self.source {
            SourceElement::Expression(e) => [(0usize, e.as_ref())].into_iter().chain(path).nth(idx),
            _ => path.nth(idx),
        };
        if let Some((real_idx, _)) = real_idx {
            if idx == 0 && matches!(self.source, SourceElement::Expression(_)) {
                self.source = SourceElement::Expression(Box::new(item));
            } else {
                self.path[real_idx - add] = SelectorElement::Expression(Box::new(item));
            }
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
            if let ExpressionType::Lambda(lambda) = expr.as_ref() {
                return Err(BuildError::unexpected_lambda(&lambda.span));
            }
        }
        for item in &path {
            if let SelectorElement::Expression(expr) = &item {
                if let ExpressionType::Lambda(lambda) = expr.as_ref() {
                    return Err(BuildError::unexpected_lambda(&lambda.span));
                }
            }
        }
        Ok(Self { source, path, span })
    }

    fn resolve_by_reference<'a: 'c, 'b, 'c>(
        &'a self,
        source: &'c Value,
        state: &'b ExpressionExecutionState<'c, 'b>,
    ) -> Result<ResolveResult<'c>, TransformError> {
        let mut elem = source;
        for p in self.path.iter() {
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
                                ))
                            }
                        },
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
        state: &ExpressionExecutionState<'_, '_>,
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

    fn resolve_by_value(
        &self,
        source: Value,
        state: &ExpressionExecutionState<'_, '_>,
    ) -> Result<ResolveResult<'_>, TransformError> {
        let mut elem = source;
        for p in self.path.iter() {
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
                                ))
                            }
                        },
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
