use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use crate::{
    compiler::BuildError,
    types::{Array, Type},
    write_list,
};

use super::{
    base::ExpressionMeta, transform_error::TransformError, Expression, ExpressionExecutionState,
    ExpressionType, ResolveResult,
};

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Expression(ExpressionType),
    Concat(ExpressionType),
}

impl Display for ArrayElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expression(x) => write!(f, "{x}"),
            Self::Concat(x) => write!(f, "...{x}"),
        }
    }
}

#[derive(Debug, Clone)]
/// Array expression. This contains a list of expressions and returns an array.
pub struct ArrayExpression {
    items: Vec<ArrayElement>,
    span: Span,
}

impl Display for ArrayExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        write_list!(f, self.items.iter());
        write!(f, "]")?;
        Ok(())
    }
}

impl Expression for ArrayExpression {
    fn resolve<'a>(
        &'a self,
        state: &mut ExpressionExecutionState<'a, '_>,
    ) -> Result<ResolveResult<'a>, TransformError> {
        state.inc_op()?;

        let mut arr = vec![];
        for expr in self.items.iter() {
            match expr {
                ArrayElement::Expression(x) => arr.push(x.resolve(state)?.into_owned()),
                ArrayElement::Concat(x) => {
                    let conc = x.resolve(state)?;
                    match conc {
                        ResolveResult::Owned(Value::Array(x)) => {
                            for elem in x {
                                arr.push(elem);
                            }
                        }
                        ResolveResult::Borrowed(Value::Array(x)) => {
                            for elem in x {
                                arr.push(elem.to_owned());
                            }
                        }
                        x => {
                            return Err(TransformError::new_incorrect_type(
                                "array",
                                "array",
                                TransformError::value_desc(&x),
                                &self.span,
                            ))
                        }
                    };
                }
            }
        }
        Ok(ResolveResult::Owned(Value::Array(arr)))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let mut types = vec![];
        let mut end_dynamic: Option<Type> = None;
        // When adding items to an array type, we either know the exact elements of the array,
        // in which case we simply add them to the list of types, or we have a dynamic end,
        // in which case we union each new type with the dynamic end type.
        for item in &self.items {
            match item {
                ArrayElement::Expression(x) => {
                    if let Some(dynamic) = end_dynamic {
                        end_dynamic = Some(dynamic.union_with(x.resolve_types(state)?));
                    } else {
                        types.push(x.resolve_types(state)?);
                    }
                }
                ArrayElement::Concat(x) => {
                    let ty = x.resolve_types(state)?;
                    // If this is valid, it must be a sequence type.
                    let seq = ty.try_as_array(&self.span)?;
                    // Just chain the elements of the sequence.
                    if let Some(mut dynamic) = end_dynamic {
                        for ty in seq.all_elements() {
                            dynamic = dynamic.union_with(ty.clone());
                        }
                        end_dynamic = Some(dynamic);
                    } else {
                        types.extend(seq.elements);
                        end_dynamic = seq.end_dynamic.map(|x| *x);
                    }
                }
            };
        }
        Ok(Type::Array(Array {
            elements: types,
            end_dynamic: end_dynamic.map(Box::new),
        }))
    }
}

impl ExpressionMeta for ArrayExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.items.iter_mut().map(|e| match e {
            ArrayElement::Expression(x) => x,
            ArrayElement::Concat(x) => x,
        }))
    }
}

impl ArrayExpression {
    pub fn new(items: Vec<ArrayElement>, span: Span) -> Result<Self, BuildError> {
        for item in &items {
            let expr = match item {
                ArrayElement::Expression(x) => x,
                ArrayElement::Concat(x) => x,
            };
            expr.fail_if_lambda()?;
        }
        Ok(Self { items, span })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compile_expression,
        tests::compile_err,
        types::{Array, Type},
    };

    #[test]
    fn test_invalid_concat() {
        let err = compile_err("[1, ...2]", &[]);
        assert_eq!(
            err.to_string(),
            "Compilation failed: array. Got number, expected array at 0..9"
        );
    }

    #[test]
    fn test_array_types_known() {
        let expr = compile_expression(
            "[1, 'a', ...[true, false], ...[1, 2, 3], input]",
            &["input"],
        )
        .unwrap();
        let ty = expr.run_types([Type::Integer]).unwrap();
        assert_eq!(
            ty,
            Type::Array(Array {
                elements: vec![
                    Type::from_const(1),
                    Type::from_const("a"),
                    Type::from_const(true),
                    Type::from_const(false),
                    Type::from_const(1),
                    Type::from_const(2),
                    Type::from_const(3),
                    Type::Integer,
                ],
                end_dynamic: None,
            })
        );
        assert_eq!(ty.to_string(), "[1, \"a\", true, false, 1, 2, 3, Integer]");
    }

    #[test]
    fn test_array_types_dynamic() {
        let expr = compile_expression("[1, ...input, ...context]", &["input", "context"]).unwrap();
        let ty = expr
            .run_types([
                Type::Array(Array {
                    elements: vec![],
                    end_dynamic: Some(Box::new(Type::Integer)),
                }),
                Type::Array(Array {
                    elements: vec![
                        Type::from_const(1),
                        Type::from_const(2),
                        Type::from_const("a"),
                    ],
                    end_dynamic: Some(Box::new(Type::Float)),
                }),
            ])
            .unwrap();
        assert_eq!(
            ty,
            Type::Array(Array {
                elements: vec![Type::from_const(1)],
                end_dynamic: Some(Box::new(
                    Type::Integer
                        .union_with(Type::from_const("a"))
                        .union_with(Type::Float)
                )),
            })
        );
        assert_eq!(ty.to_string(), "[1, ...Union<Integer, \"a\", Float>]");
    }
}
