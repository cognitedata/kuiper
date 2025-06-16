use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::{
    expressions::{functions::LambdaAcceptFunction, Expression, ResolveResult},
    types::{Object, Sequence, Type},
    BuildError, TransformError,
};

function_def!(DistinctByFunction, "distinct_by", 2, lambda);

impl<'a: 'c, 'c> Expression<'a, 'c> for DistinctByFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<crate::expressions::ResolveResult<'c>, crate::TransformError> {
        let source = self.args[0].resolve(state)?;

        match source.as_ref() {
            Value::Array(x) => {
                let mut res: Vec<Value> = Vec::new();
                let mut found: HashSet<String> = HashSet::new();
                for val in x {
                    let res_inner = self.args[1].call(state, &[val])?;
                    if found.insert(res_inner.to_string()) {
                        res.push(val.to_owned());
                    }
                }
                Ok(ResolveResult::Owned(Value::Array(res)))
            }
            Value::Object(x) => {
                let mut res: Map<String, Value> = Map::new();
                let mut found: HashSet<String> = HashSet::new();
                for (k, v) in x {
                    let res_inner = self.args[1].call(state, &[v, &Value::String(k.to_owned())])?;
                    if found.insert(res_inner.to_string()) {
                        res.insert(k.to_owned(), v.to_owned());
                    }
                }
                Ok(ResolveResult::Owned(Value::Object(res)))
            }
            x => Err(TransformError::new_incorrect_type(
                "Incorrect input to distinct_by",
                "array or object",
                TransformError::value_desc(x),
                &self.span,
            )),
        }
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let source = self.args[0].resolve_types(state)?;
        let mut res_type = Type::never();
        let mut allows_object = false;

        if let Ok(obj) = source.try_as_object(&self.span) {
            for (k, v) in obj.fields.iter() {
                match k {
                    crate::types::ObjectField::Constant(r) => {
                        self.args[1]
                            .call_types(state, &[v, &Type::from_const(Value::String(r.clone()))])?;
                    }
                    crate::types::ObjectField::Generic => {
                        self.args[1].call_types(state, &[v, &Type::String])?;
                    }
                }
            }
            res_type = res_type.union_with(Type::Object(Object {
                fields: [(crate::types::ObjectField::Generic, obj.element_union())]
                    .into_iter()
                    .collect(),
            }));
            allows_object = true;
        }

        if let Ok(arr) = source.try_as_array(&self.span) {
            for item in &arr.elements {
                self.args[1].call_types(
                    state,
                    &[
                        item,
                        &if allows_object {
                            Type::String
                        } else {
                            Type::null()
                        },
                    ],
                )?;
            }

            res_type = res_type.union_with(Type::Sequence(Sequence {
                elements: vec![],
                end_dynamic: Some(Box::new(arr.element_union())),
            }));
        }

        Ok(res_type.flatten_union())
    }
}

impl LambdaAcceptFunction for DistinctByFunction {
    fn validate_lambda(
        idx: usize,
        lambda: &crate::expressions::LambdaExpression,
        _num_args: usize,
    ) -> Result<(), BuildError> {
        if idx != 1 {
            return Err(BuildError::unexpected_lambda(&lambda.span));
        }
        let nargs = lambda.input_names.len();
        if !(1..=2).contains(&nargs) {
            return Err(BuildError::n_function_args(
                lambda.span.clone(),
                "distict_by takes a function with one or two arguments",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use logos::Span;

    use crate::{compile_expression, CompileError, TransformError};

    #[test]
    fn test_distinct_by_fails_for_unknown_types() {
        match compile_expression(r#"distinct_by(1234567890, (a) => a)"#, &[]) {
            Ok(_) => panic!("Should not be able to resolve"),
            Err(err) => match err {
                CompileError::Optimizer(TransformError::IncorrectTypeInField(t_err)) => {
                    assert_eq!(
                        t_err.desc,
                        "Incorrect input to distinct_by. Got number, expected array or object"
                    );
                    assert_eq!(t_err.span, Span { start: 0, end: 33 })
                }
                _ => panic!("Should be an optimizer error"),
            },
        }
    }

    #[test]
    fn test_distinct_by_for_arrays() {
        let expr =
            compile_expression(r#"distinct_by(["sheep", "apple", "sheep"], a => a)"#, &[]).unwrap();

        let res = expr.run([]).unwrap();

        let val_arr = res.as_array().unwrap();
        assert_eq!(val_arr.len(), 2);
        assert_eq!(val_arr.first().unwrap(), "sheep");
        assert_eq!(val_arr.get(1).unwrap(), "apple");
    }

    #[test]
    fn test_distinct_by_for_objects() {
        let expr = compile_expression(
            r#"distinct_by({'x': 'y', 'a': 'b', 'c': 'b'}, (a, b) => a)"#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();

        let val = res.as_object().unwrap();
        assert_eq!(val.len(), 2);
        assert_eq!(val.get("x").unwrap().to_owned(), "y".to_string());
        assert_eq!(val.get("a").unwrap().to_owned(), "b".to_string());
    }
}
