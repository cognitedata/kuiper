use std::fmt::Display;

use itertools::Itertools;
use logos::Span;

use crate::{write_list, BuildError};

use super::{Expression, ExpressionMeta, ExpressionType, ResolveResult};

#[derive(Debug)]
pub struct MacroCallExpression {
    pub inner: Box<ExpressionType>,
    pub args: Vec<ExpressionType>,
    pub span: Span,
}

impl Display for MacroCallExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})(", self.inner)?;
        write_list!(f, &self.args);
        write!(f, ")")
    }
}

impl MacroCallExpression {
    pub fn new(
        inner: ExpressionType,
        args: Vec<ExpressionType>,
        span: Span,
    ) -> Result<Self, BuildError> {
        for a in &args {
            a.fail_if_lambda()?;
        }
        Ok(Self {
            inner: Box::new(inner),
            args,
            span,
        })
    }
}

impl Expression for MacroCallExpression {
    fn resolve<'a: 'c, 'c>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'c, '_>,
    ) -> Result<super::ResolveResult<'c>, crate::TransformError> {
        state.inc_op()?;
        let mut args = Vec::with_capacity(self.args.len());
        for a in &self.args {
            args.push(a.resolve(state)?);
        }
        let refs = args
            .iter()
            .map(|a: &ResolveResult<'_>| a.as_ref())
            .collect_vec();
        self.inner.call(state, &refs)
    }
}

impl ExpressionMeta for MacroCallExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(
            [self.inner.as_mut()]
                .into_iter()
                .chain(self.args.iter_mut()),
        )
    }
}

#[cfg(test)]
mod tests {
    use logos::Span;
    use serde_json::Value;

    use crate::{compile_expression, BuildError, CompileError, TransformError};

    fn compile_err(data: &str, inputs: &[&str]) -> CompileError {
        match compile_expression(data, inputs) {
            Ok(_) => panic!("Expected compilation to fail"),
            Err(x) => x,
        }
    }

    #[test]
    pub fn test_simple_macro_expansion() {
        let expr = compile_expression(
            r#"
        #my_macro := (a, b) => a + b;

        1 + 2 + my_macro(3, 4) + my_macro(5, my_macro(6, 7))
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        assert_eq!(28, res.as_u64().unwrap());
    }

    #[test]
    pub fn test_recursive_macro_fail() {
        let err = compile_err(
            r#"
        #my_macro := (a, b) => my_other_macro(a, b);
        #my_other_macro := (c, d) =>  my_macro(c, d);

        my_macro(1, 1)
        "#,
            &[],
        );
        match err {
            CompileError::Build(BuildError::Other(d)) => {
                assert_eq!(d.detail, "Recursive macro calls are not allowed");
                assert_eq!(
                    d.position,
                    Span {
                        start: 92,
                        end: 106
                    }
                );
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_allowed_macro_nesting() {
        let expr = compile_expression(
            r#"
        #m1 := (a, b) => a + b;
        #m2 := (c, d, e) => m1(c, d) + m1(d, e);
        #m3 := (f, g, h, i) => m2(f, g, h) + m2(g, h, i);

        m3(1, 2, 3, 4)"#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        assert_eq!(20, res.as_u64().unwrap());
    }

    #[test]
    pub fn test_macro_error_loc() {
        // Expect to get an error inside the macro expansion.
        let expr = compile_expression(r#"#m := () => input / 0; m()"#, &["input"]).unwrap();
        let err = expr.run(&[Value::from(10)]).unwrap_err();
        match err {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 18, end: 19 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }
}
