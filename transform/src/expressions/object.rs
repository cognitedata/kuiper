use std::fmt::Display;

use logos::Span;
use serde_json::{Map, Value};

use crate::compiler::BuildError;

use super::{
    base::{get_string_from_value, ExpressionMeta},
    Expression, ExpressionType, ResolveResult,
};

#[derive(Debug, Clone)]
pub struct ObjectExpression {
    pairs: Vec<(ExpressionType, ExpressionType)>,
    span: Span,
}

impl Display for ObjectExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut needs_comma = false;
        for (key, value) in self.pairs.iter() {
            if needs_comma {
                write!(f, ", ")?;
            }
            needs_comma = true;
            write!(f, "{key}: {value}")?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for ObjectExpression {
    fn resolve(
        &'a self,
        state: &super::ExpressionExecutionState<'c, '_>,
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        let mut output = Map::with_capacity(self.pairs.len());
        for (key, value) in self.pairs.iter() {
            let key_res = key.resolve(state)?;
            let key_val = get_string_from_value("object", key_res.as_ref(), &self.span, state.id)?;
            output.insert(key_val.into_owned(), value.resolve(state)?.into_owned());
        }
        Ok(ResolveResult::Owned(Value::Object(output)))
    }
}

impl ExpressionMeta for ObjectExpression {
    fn num_children(&self) -> usize {
        self.pairs.len() * 2
    }

    fn get_child(&self, idx: usize) -> Option<&ExpressionType> {
        self.pairs
            .get(idx / 2)
            .map(|(key, value)| if idx % 2 == 0 { key } else { value })
    }

    fn get_child_mut(&mut self, idx: usize) -> Option<&mut ExpressionType> {
        self.pairs
            .get_mut(idx / 2)
            .map(|(key, value)| if idx % 2 == 0 { key } else { value })
    }

    fn set_child(&mut self, idx: usize, item: ExpressionType) {
        if idx >= self.pairs.len() * 2 {
            return;
        }
        let pair = self.pairs.get_mut(idx / 2).unwrap();
        if idx % 2 == 0 {
            pair.0 = item;
        } else {
            pair.1 = item;
        }
    }
}

impl ObjectExpression {
    pub fn new(
        pairs: Vec<(ExpressionType, ExpressionType)>,
        span: Span,
    ) -> Result<Self, BuildError> {
        for (it1, it2) in &pairs {
            if let ExpressionType::Lambda(lambda) = &it1 {
                return Err(BuildError::unexpected_lambda(&lambda.span));
            }
            if let ExpressionType::Lambda(lambda) = &it2 {
                return Err(BuildError::unexpected_lambda(&lambda.span));
            }
        }
        Ok(Self { pairs, span })
    }
}
