use std::fmt::Display;

use logos::Span;
use serde_json::{Map, Value};

use crate::{compiler::BuildError, write_list, TransformError};

use super::{base::ExpressionMeta, Expression, ExpressionType, ResolveResult};

#[derive(Debug, Clone)]
pub enum ObjectElement {
    Pair(ExpressionType, ExpressionType),
    Concat(ExpressionType),
}

impl Display for ObjectElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(key, value) => write!(f, "{key}: {value}"),
            Self::Concat(x) => write!(f, "..{x}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectExpression {
    items: Vec<ObjectElement>,
    span: Span,
}

impl Display for ObjectExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        write_list!(f, &self.items);
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
                    let key_val = key_res.try_into_string("object", &self.span)?;
                    output.insert(key_val.into_owned(), value.resolve(state)?.into_owned());
                }
                ObjectElement::Concat(x) => {
                    let conc = x.resolve(state)?;
                    match conc {
                        ResolveResult::Owned(Value::Object(x)) => {
                            for (k, v) in x {
                                output.entry(k).or_insert(v);
                            }
                        }
                        ResolveResult::Borrowed(Value::Object(x)) => {
                            for (k, v) in x {
                                output.entry(k.to_owned()).or_insert_with(|| v.to_owned());
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
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.items.iter_mut().flat_map(|f| match f {
            ObjectElement::Pair(x, y) => vec![x, y].into_iter(),
            ObjectElement::Concat(x) => vec![x].into_iter(),
        }))
    }
}

impl ObjectExpression {
    pub fn new(items: Vec<ObjectElement>, span: Span) -> Result<Self, BuildError> {
        for k in &items {
            match k {
                ObjectElement::Pair(key, val) => {
                    key.fail_if_lambda()?;
                    val.fail_if_lambda()?;
                }
                ObjectElement::Concat(x) => {
                    x.fail_if_lambda()?;
                }
            }
        }
        Ok(Self { items, span })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::compile_expression;

    #[test]
    fn test_object_expression() {
        let expr = compile_expression(
            r#"{
                "a": 1,
                "b": 2,
                ...{ "c": 3 },
                "d": 4,
                ...{
                    "a": 5,
                    "e": 5
                },
                "e": 6,
            }
            "#,
            &[],
        )
        .unwrap();
        let val = expr.run([]).unwrap();
        assert_eq!(
            val.as_ref(),
            &json!(
                {
                    "a": 1,
                    "b": 2,
                    "c": 3,
                    "d": 4,
                    "e": 6
                }
            )
        );
    }
}
