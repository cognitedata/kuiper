use std::{borrow::Cow, collections::HashMap, fmt::Display};

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
    Input(String),
    CompiledInput(usize),
    Expression(Box<ExpressionType>),
}

impl From<SelectorElement> for SourceElement {
    fn from(value: SelectorElement) -> Self {
        match value {
            SelectorElement::Constant(x) => Self::Input(x),
            SelectorElement::Expression(x) => Self::Expression(x),
        }
    }
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
            SourceElement::Input(s) => write!(f, "{s}"),
            SourceElement::CompiledInput(s) => write!(f, "${s}"),
            SourceElement::Expression(e) => write!(f, "{e}"),
        }
    }
}

impl Display for SelectorExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)?;
        for el in &self.path {
            if matches!(el, SelectorElement::Constant(_)) {
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
            SourceElement::Input(s) => Err(TransformError::new_source_missing(
                s.to_string(),
                &self.span,
            )),
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

    pub fn resolve_first_item(
        &mut self,
        _state: &ExpressionExecutionState<'_, '_>,
        map: &HashMap<String, usize>,
    ) -> Result<(), TransformError> {
        let new_source = match &self.source {
            SourceElement::Input(x) => map
                .get(x)
                .copied()
                .ok_or_else(|| TransformError::new_source_missing(x.to_string(), &self.span))?,
            _ => return Ok(()),
        };
        self.source = SourceElement::CompiledInput(new_source);
        Ok(())
    }
}
