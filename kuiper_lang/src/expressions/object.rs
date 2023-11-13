use std::fmt::Display;

use logos::Span;
use serde_json::{Map, Value};

use crate::{compiler::BuildError, TransformError};

use super::{
    base::{get_string_from_cow_value, ExpressionMeta},
    Expression, ExpressionType, ResolveResult,
};

#[derive(Debug, Clone)]
pub enum ObjectElement {
    Pair(ExpressionType, ExpressionType),
    Concat(ExpressionType),
}

#[derive(Debug, Clone)]
pub struct ObjectExpression {
    items: Vec<ObjectElement>,
    span: Span,
}

impl Display for ObjectExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut needs_comma = false;
        for k in self.items.iter() {
            if needs_comma {
                write!(f, ", ")?;
            }
            needs_comma = true;
            match k {
                ObjectElement::Pair(key, value) => write!(f, "{key}: {value}")?,
                ObjectElement::Concat(x) => write!(f, "..{x}")?,
            }
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for ObjectExpression {
    fn resolve(
        &'a self,
        state: &mut super::ExpressionExecutionState<'c, '_>,
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        state.inc_op()?;
        let mut output = Map::with_capacity(self.items.len());
        for k in self.items.iter() {
            match k {
                ObjectElement::Pair(key, value) => {
                    let key_res = key.resolve(state)?;
                    let key_val = get_string_from_cow_value("object", key_res, &self.span)?;
                    output.insert(key_val.into_owned(), value.resolve(state)?.into_owned());
                }
                ObjectElement::Concat(x) => {
                    let conc = x.resolve(state)?;
                    match conc {
                        ResolveResult::Owned(Value::Object(x)) => {
                            for (k, v) in x {
                                output.insert(k, v);
                            }
                        }
                        ResolveResult::Borrowed(Value::Object(x)) => {
                            for (k, v) in x {
                                output.insert(k.to_owned(), v.to_owned());
                            }
                        }
                        x => {
                            return Err(TransformError::new_incorrect_type(
                                "object",
                                "object",
                                TransformError::value_desc(&x),
                                &self.span,
                            ))
                        }
                    };
                }
            }
        }
        Ok(ResolveResult::Owned(Value::Object(output)))
    }
}

impl ExpressionMeta for ObjectExpression {
    fn iter_children(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.items.iter_mut().flat_map(|f| match f {
            ObjectElement::Pair(x, y) => vec![x, y].into_iter(),
            ObjectElement::Concat(x) => vec![x].into_iter(),
        }))
    }

    fn num_children(&self) -> usize {
        let mut len = 0;
        for it in &self.items {
            match it {
                ObjectElement::Pair(_, _) => len += 2,
                ObjectElement::Concat(_) => len += 1,
            }
        }
        len
    }

    fn get_child(&self, mut idx: usize) -> Option<&ExpressionType> {
        for it in &self.items {
            match it {
                ObjectElement::Pair(x, y) => {
                    if idx == 0 {
                        return Some(x);
                    } else if idx == 1 {
                        return Some(y);
                    } else {
                        idx -= 2;
                    }
                }
                ObjectElement::Concat(x) => {
                    if idx == 0 {
                        return Some(x);
                    } else {
                        idx -= 1;
                    }
                }
            }
        }
        None
    }

    fn get_child_mut(&mut self, mut idx: usize) -> Option<&mut ExpressionType> {
        for it in &mut self.items {
            match it {
                ObjectElement::Pair(x, y) => {
                    if idx == 0 {
                        return Some(x);
                    } else if idx == 1 {
                        return Some(y);
                    } else {
                        idx -= 2;
                    }
                }
                ObjectElement::Concat(x) => {
                    if idx == 0 {
                        return Some(x);
                    } else {
                        idx -= 1;
                    }
                }
            }
        }
        None
    }

    fn set_child(&mut self, mut idx: usize, item: ExpressionType) {
        if idx >= self.items.len() * 2 {
            return;
        }
        for it in &mut self.items {
            match it {
                ObjectElement::Pair(x, y) => {
                    if idx == 0 {
                        *x = item;
                        return;
                    } else if idx == 1 {
                        *y = item;
                        return;
                    } else {
                        idx -= 2;
                    }
                }
                ObjectElement::Concat(x) => {
                    if idx == 0 {
                        *x = item;
                        return;
                    } else {
                        idx -= 1;
                    }
                }
            }
        }
    }
}

impl ObjectExpression {
    pub fn new(items: Vec<ObjectElement>, span: Span) -> Result<Self, BuildError> {
        for k in &items {
            match k {
                ObjectElement::Pair(key, val) => {
                    if let ExpressionType::Lambda(lambda) = key {
                        return Err(BuildError::unexpected_lambda(&lambda.span));
                    }
                    if let ExpressionType::Lambda(lambda) = val {
                        return Err(BuildError::unexpected_lambda(&lambda.span));
                    }
                }
                ObjectElement::Concat(x) => {
                    if let ExpressionType::Lambda(lambda) = x {
                        return Err(BuildError::unexpected_lambda(&lambda.span));
                    }
                }
            }
        }
        Ok(Self { items, span })
    }
}
