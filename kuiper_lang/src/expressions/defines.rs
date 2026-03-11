use std::fmt::Display;

use crate::{Expression, ExpressionMeta, ExpressionType, ResolveResult};

#[derive(Debug)]
pub struct DefineExpression {
    pub defines: Vec<(String, ExpressionType)>,
    pub inner: Box<ExpressionType>,
}

impl Display for DefineExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, expr) in &self.defines {
            write!(f, "#{} := {};", name, expr)?;
        }
        write!(f, "{}", self.inner)?;
        Ok(())
    }
}

impl Expression for DefineExpression {
    fn resolve<'a>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'a, '_>,
    ) -> Result<super::ResolveResult<'a>, super::TransformError> {
        let mut inner = state.get_empty_temp_clone();

        // We need each definition to be available to each subsequent definition,
        // so we resolve them one by one, pushing the results into the temporary state as we go.
        // Since the temporary state needs to contain _references_ to the data,
        // we first allocate storage for that data, then mutable split off the first element
        // to get a mutable reference to it, which we can then push into the temporary state.

        let mut data = vec![None; self.defines.len()];
        let mut data_ref: &mut [Option<ResolveResult<'_>>] = &mut data[..];
        for (_, expr) in self.defines.iter() {
            let mut state = inner.get_temp_state();
            let res = expr.resolve(&mut state)?;

            // Unwrap is safe because we split off one element for each definition, and there are as many elements as definitions.
            let (item, next_data_ref) = data_ref.split_first_mut().unwrap();
            data_ref = next_data_ref;

            // This would use `insert`, but the borrow checker breaks down on the reborrow.
            item.replace(res);
            let v = item.as_ref().unwrap();
            inner.push_data(v.as_ref());
        }

        let mut state = inner.get_temp_state();
        let r = self.inner.resolve(&mut state)?;
        Ok(super::ResolveResult::Owned(r.into_owned()))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let mut types = vec![None; self.defines.len()];
        let mut inner = state.get_empty_temp_clone();

        let mut types_ref: &mut [Option<crate::types::Type>] = &mut types[..];
        for (_, expr) in &self.defines {
            let mut state = inner.get_temp_state();
            let res = expr.resolve_types(&mut state)?;
            let (item, next_types) = types_ref.split_first_mut().unwrap();
            types_ref = next_types;

            item.replace(res);
            let v = item.as_ref().unwrap();
            inner.push_data(v);
        }
        let mut state = inner.get_temp_state();
        self.inner.resolve_types(&mut state)
    }
}

impl ExpressionMeta for DefineExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(
            self.defines
                .iter_mut()
                .map(|(_, expr)| expr)
                .chain(std::iter::once(self.inner.as_mut())),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile_expression, types::Type, BuildError};

    #[test]
    fn test_defines_outer() {
        let expr = compile_expression(
            r#"
        #foo := 1 + 1;

        foo + 5
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap().into_owned();
        assert_eq!(res, 7);
    }

    #[test]
    fn test_multiple_defines() {
        let expr = compile_expression(
            r#"
        #foo := 1 + 1;
        #bar := foo * 2;

        foo + bar
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap().into_owned();
        assert_eq!(res, 6);
    }

    #[test]
    fn test_defines_inner() {
        let expr = compile_expression(
            r#"
        [1, 2, 3].map(a =>
            #foo := a * 2;
            foo + 1
        )
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap().into_owned();
        assert_eq!(res, serde_json::json!([3, 5, 7]));
    }

    #[test]
    fn test_multiple_defines_inner() {
        let expr = compile_expression(
            r#"
        [1, 2, 3].map(a =>
            #foo := a * 2;
            #bar := foo + 1;
            bar * 2
        )
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap().into_owned();
        assert_eq!(res, serde_json::json!([6, 10, 14]));
    }

    #[test]
    fn test_define_name_conflict() {
        let err = compile_expression(
            r#"
        #foo := 1;
        #foo := 2;

        foo
        "#,
            &[],
        )
        .unwrap_err();
        match err {
            crate::CompileError::Build(BuildError::VariableConflict(e)) => {
                assert_eq!(e.detail, "foo");
            }
            _ => panic!("Unexpected error: {err}"),
        }
    }

    #[test]
    fn test_define_name_conflict_inner() {
        let err = compile_expression(
            r#"
        [1, 2, 3].map(a =>
            #foo := a;
            #foo := a * 2;
            foo
        )
        "#,
            &[],
        )
        .unwrap_err();
        match err {
            crate::CompileError::Build(BuildError::VariableConflict(e)) => {
                assert_eq!(e.detail, "foo");
            }
            _ => panic!("Unexpected error: {err}"),
        }
    }

    #[test]
    fn test_complex_define() {
        let expr = compile_expression(
            r#"
        [1, 2, 3].map(a =>
            #foo := { "a": a, "b": input };
            #bar := foo.a + foo.b;
            bar * 2
        )
        "#,
            &["input"],
        )
        .unwrap();

        let res = expr.run([&serde_json::json!(10)]).unwrap().into_owned();
        assert_eq!(res, serde_json::json!([22, 24, 26]));
    }

    #[test]
    fn test_nested_define() {
        let expr = compile_expression(
            r#"
        #foo := 1;
        #bar := foo + 1;
        [1, 2, 3].map(a =>
            // Comments are allowed
            #baz := a * 2;
            // Between variables
            #quz := foo + bar + baz;
            quz
        ).map(a =>
            #baz := a * 2;
            baz
        )
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap().into_owned();
        assert_eq!(res, serde_json::json!([10, 14, 18]));
    }

    #[test]
    fn test_template_string_display() {
        let expr = compile_expression(
            r#"
            #foo := 1;
            input.map(a =>
                #bar := a * foo;
                bar
            )
            "#,
            &["input"],
        )
        .unwrap();
        let display = format!("{}", expr);
        assert_eq!(display, r#"#foo := 1;map($0, (a) => #bar := ($2 * $1);$3)"#);
    }

    #[test]
    fn test_template_string_types() {
        let expr = compile_expression(
            r#"
            #foo := input + 1;
            #bar := string(foo);
            concat(bar, 'A')
            "#,
            &["input"],
        )
        .unwrap();

        let ty = expr.run_types([Type::Integer]).unwrap();
        assert_eq!(ty, Type::String);
    }
}
