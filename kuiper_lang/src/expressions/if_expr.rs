use std::fmt::Display;

use logos::Span;
use serde_json::Value;

use crate::{
    types::{Truthy, Type},
    ExpressionType,
};

use super::{Expression, ExpressionMeta};

#[derive(Debug)]
pub struct IfExpression {
    args: Vec<ExpressionType>,
    #[allow(unused)]
    span: Span,
}

impl Display for IfExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "if ")?;
        write!(f, "{} {{ {} }}", self.args[0], self.args[1])?;

        let mut iter = self.args.iter().skip(2);
        loop {
            let a1 = iter.next();
            let a2 = iter.next();

            match (a1, a2) {
                (Some(a1), Some(a2)) => write!(f, " else if {a1} {{ {a2} }}")?,
                (Some(a1), None) => write!(f, " else {{ {a1} }}")?,
                _ => break,
            }
        }

        Ok(())
    }
}

impl Expression for IfExpression {
    fn resolve<'a>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'a, '_>,
    ) -> Result<super::ResolveResult<'a>, crate::TransformError> {
        state.inc_op()?;
        let mut iter = self.args.iter();

        loop {
            let a1 = iter.next();
            let a2 = iter.next();

            match (a1, a2) {
                (Some(a1), Some(a2)) => {
                    let cond = a1.resolve(state)?.as_bool();
                    if cond {
                        break a2.resolve(state);
                    }
                }
                (Some(a1), None) => {
                    break a1.resolve(state);
                }
                _ => {
                    break Ok(super::ResolveResult::Owned(serde_json::Value::Null));
                }
            }
        }
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<Type, crate::types::TypeError> {
        let mut final_type = Type::never();

        let mut iter = self.args.iter();
        loop {
            let a1 = iter.next();
            let a2 = iter.next();

            match (a1, a2) {
                (Some(a1), Some(a2)) => {
                    let cond = a1.resolve_types(state)?.truthyness();
                    match cond {
                        Truthy::Always => {
                            final_type = final_type.union_with(a2.resolve_types(state)?);
                            break;
                        }
                        Truthy::Never => {
                            continue;
                        }
                        Truthy::Maybe => {
                            final_type = final_type.union_with(a2.resolve_types(state)?);
                        }
                    }
                }
                (Some(a1), None) => {
                    final_type = final_type.union_with(a1.resolve_types(state)?);
                    break;
                }
                _ => {
                    final_type = final_type.union_with(Type::Constant(Value::Null));
                    break;
                }
            }
        }

        Ok(final_type)
    }
}

impl IfExpression {
    pub fn new(args: Vec<ExpressionType>, span: Span) -> Self {
        Self { args, span }
    }
}

impl ExpressionMeta for IfExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.args.iter_mut())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{compile_expression, types::Type};

    #[test]
    fn test_if_expr() {
        let expr = compile_expression(
            r#"
            if (input > 2) {
                15
            } else if input == 2 {
                25
            } else {
                35
            }
            "#,
            &["input"],
        )
        .unwrap();
        let v = Value::from(2);
        let r = expr.run([&v]).unwrap();
        assert_eq!(r.into_owned(), Value::from(25));
        let v = Value::from(3);
        let r = expr.run([&v]).unwrap();
        assert_eq!(r.into_owned(), Value::from(15));
        let v = Value::from(1);
        let r = expr.run([&v]).unwrap();
        assert_eq!(r.into_owned(), Value::from(35));
    }

    #[test]
    fn test_if_without_else() {
        let expr = compile_expression(
            r#"
            if input > 2 {
                15
            } else if input == 2 {
                25
            }
            "#,
            &["input"],
        )
        .unwrap();
        let v = Value::from(2);
        let r = expr.run([&v]).unwrap();
        assert_eq!(r.into_owned(), Value::from(25));
        let v = Value::from(3);
        let r = expr.run([&v]).unwrap();
        assert_eq!(r.into_owned(), Value::from(15));
        let v = Value::from(1);
        let r = expr.run([&v]).unwrap();
        assert_eq!(r.into_owned(), Value::Null);
    }

    #[test]
    fn test_if_types() {
        let expr = compile_expression(
            r#"
            if input > 2 {
                15
            } else if input == 2 {
                "25"
            } else {
                true
            }
            "#,
            &["input"],
        )
        .unwrap();
        let ty = expr.run_types([crate::types::Type::Integer]).unwrap();
        assert_eq!(
            ty,
            Type::from_const(15)
                .union_with(Type::from_const("25"))
                .union_with(Type::from_const(true))
        );
    }

    #[test]
    fn test_if_types_known_conditions() {
        let expr = compile_expression(
            r#"
            if false {
                15
            } else if input {
                "25"
            } else {
                true
            }
            "#,
            &["input"],
        )
        .unwrap();
        let ty = expr.run_types([crate::types::Type::Integer]).unwrap();
        assert_eq!(ty, Type::from_const("25"));
    }

    #[test]
    fn test_if_types_no_else() {
        let expr = compile_expression(
            r#"
            if input == 2 {
                15
            } else if input > 2 {
                "25"
            }
            "#,
            &["input"],
        )
        .unwrap();
        let ty = expr.run_types([Type::Integer]).unwrap();
        assert_eq!(
            ty,
            Type::from_const(15)
                .union_with(Type::from_const("25"))
                .union_with(Type::null())
        );
    }
}
