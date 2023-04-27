use std::{borrow::Cow, collections::HashMap, fmt::Display};

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
    CompiledConstant(usize),
    Expression(Box<ExpressionType>),
}

impl Display for SelectorElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectorElement::Constant(x) => write!(f, "{x}"),
            SelectorElement::Expression(x) => write!(f, "[{x}]"),
            SelectorElement::CompiledConstant(x) => write!(f, "{x}"),
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
                    SelectorElement::CompiledConstant(x) => match state.get_value(*x) {
                        Some(x) => x,
                        None => {
                            return Err(TransformError::new_source_missing(
                                x.to_string(),
                                &self.span,
                                state.id,
                            ))
                        }
                    },
                    x => {
                        return Err(TransformError::new_source_missing(
                            x.to_string(),
                            &self.span,
                            state.id,
                        ))
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
                SelectorElement::CompiledConstant(_) => unreachable!(),
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
                SelectorElement::CompiledConstant(_) => unreachable!(),
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

    pub fn resolve_first_item(
        &mut self,
        state: &ExpressionExecutionState<'_, '_>,
        map: &HashMap<String, usize>,
    ) -> Result<(), TransformError> {
        if !matches!(self.source, SourceElement::Input) {
            return Ok(());
        }

        let first_sel = self.path.first().unwrap();
        let new_first = match first_sel {
            SelectorElement::Constant(x) => map.get(x).copied().ok_or_else(|| {
                TransformError::new_source_missing(x.to_string(), &self.span, state.id)
            })?,
            SelectorElement::CompiledConstant(_) => return Ok(()),
            SelectorElement::Expression(x) => match x.resolve(state) {
                Ok(r) => match r.as_ref() {
                    Value::String(s) => map.get(s).copied().ok_or_else(|| {
                        TransformError::new_source_missing(x.to_string(), &self.span, state.id)
                    })?,
                    d => {
                        return Err(TransformError::new_incorrect_type(
                            "First selector from input must be a string",
                            "String",
                            TransformError::value_desc(d),
                            &self.span,
                            state.id,
                        ))
                    }
                },
                Err(TransformError::SourceMissingError(_)) => {
                    return Err(TransformError::new_invalid_operation(
                        "First selector from source must be a constant".to_string(),
                        &self.span,
                        state.id,
                    ))
                }
                Err(e) => {
                    return Err(e);
                }
            },
        };
        self.path[0] = SelectorElement::CompiledConstant(new_first);

        Ok(())
    }
}
