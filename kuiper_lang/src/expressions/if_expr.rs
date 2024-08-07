use std::fmt::Display;

use logos::Span;

use crate::ExpressionType;

use super::{Expression, ExpressionMeta};

#[derive(Debug, Clone)]
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
                (Some(a1), Some(a2)) => write!(f, "else if {} {{ {} }}", a1, a2)?,
                (Some(a1), None) => write!(f, "else {{ {} }}", a1)?,
                _ => break,
            }
        }

        Ok(())
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for IfExpression {
    fn resolve(
        &'a self,
        state: &mut super::ExpressionExecutionState<'c, '_>,
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
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

    use crate::compile_expression;

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
}
