use serde_json::{Map, Value};

use crate::expressions::functions::FunctionExpression;
use crate::expressions::{Expression, ResolveResult};
use crate::NULL_CONST;

macro_rules! regex_function {
    ($typ:ident, $name:expr, $nargs:expr) => {
        #[derive(Debug, Clone)]
        pub struct $typ {
            args: [Box<$crate::expressions::ExpressionType>; $nargs],
            span: logos::Span,
            re: regex::Regex,
        }

        impl std::fmt::Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}({}, {}",
                    <Self as $crate::expressions::functions::FunctionExpression>::INFO.name,
                    &self.args[0],
                    self.re
                )?;
                for arg in self.args.iter().skip(1) {
                    write!(f, ", {}", arg)?;
                }
                write!(f, ")")
            }
        }

        impl $crate::expressions::functions::LambdaAcceptFunction for $typ {}

        impl $crate::expressions::functions::FunctionExpression for $typ {
            const INFO: $crate::expressions::functions::FunctionInfo =
                $crate::expressions::functions::FunctionInfo {
                    minargs: 2,
                    maxargs: Some($nargs + 1),
                    name: $name,
                };
            fn new(
                args: Vec<$crate::expressions::ExpressionType>,
                span: logos::Span,
            ) -> Result<Self, crate::BuildError> {
                // Require the regex to be constant

                if !Self::INFO.validate(args.len()) {
                    return Err($crate::BuildError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                let num_args = args.len();
                for (idx, arg) in args.iter().enumerate() {
                    if let $crate::expressions::ExpressionType::Lambda(lambda) = arg {
                        <Self as $crate::expressions::functions::LambdaAcceptFunction>::validate_lambda(idx, lambda, num_args)?;
                    }
                }
                let mut final_args = Vec::new();
                let mut arg_iter = args.into_iter();
                final_args.push(Box::new(arg_iter.next().unwrap()));
                let regex_arg = arg_iter.next().unwrap();
                final_args.extend(arg_iter.map(|a| Box::new(a)));

                let $crate::expressions::ExpressionType::Constant(c) = &regex_arg else {
                    return Err($crate::BuildError::other(span.clone(), "Regex must be constant at compile time"));
                };
                let serde_json::Value::String(r) = c.value() else {
                    return Err($crate::BuildError::other(span.clone(), "Regex must be constant at compile time"));
                };
                let re = regex::Regex::new(r.as_ref()).map_err(|e| {
                    $crate::BuildError::other(span.clone(), &format!("Regex compilation failed: {e}"))
                })?;
                Ok(Self {
                    span,
                    args: final_args.try_into().unwrap(),
                    re,
                })
            }
        }

        impl $crate::expressions::ExpressionMeta for $typ {
            fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut $crate::expressions::ExpressionType> + '_> {
                Box::new(self.args.iter_mut().map(|m| m.as_mut()))
            }
        }
    };
}

regex_function!(RegexIsMatchFunction, "regex_is_match", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for RegexIsMatchFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let arg = self.args[0].resolve(state)?;
        let arg = arg.try_as_string(Self::INFO.name, &self.span)?;
        Ok(ResolveResult::Owned(self.re.is_match(arg.as_ref()).into()))
    }
}

regex_function!(RegexFirstMatchFunction, "regex_first_match", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for RegexFirstMatchFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let arg = self.args[0].resolve(state)?;
        let arg = arg.try_as_string(Self::INFO.name, &self.span)?;
        let m = self.re.find(arg.as_ref());
        Ok(ResolveResult::Owned(match m {
            Some(v) => Value::String(v.as_str().to_owned()),
            None => Value::Null,
        }))
    }
}

regex_function!(RegexMatchesFunction, "regex_matches", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for RegexMatchesFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let arg = self.args[0].resolve(state)?;
        let arg = arg.try_as_string(Self::INFO.name, &self.span)?;
        let m = self.re.find_iter(arg.as_ref());
        Ok(ResolveResult::Owned(Value::Array(
            m.map(|m| Value::String(m.as_str().to_owned())).collect(),
        )))
    }
}

regex_function!(RegexCapturesFunction, "regex_captures", 1);

impl<'a: 'c, 'c> Expression<'a, 'c> for RegexCapturesFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let arg = self.args[0].resolve(state)?;
        let arg = arg.try_as_string(Self::INFO.name, &self.span)?;
        let m = self.re.captures(arg.as_ref());
        let Some(m) = m else {
            return Ok(ResolveResult::Borrowed(&NULL_CONST));
        };
        let names = self.re.capture_names();
        let v: Map<String, Value> = m
            .iter()
            .zip(names)
            .enumerate()
            .filter_map(|(idx, (capture, name))| {
                let c = capture?;
                let name = name
                    .map(|n| n.to_owned())
                    .unwrap_or_else(|| idx.to_string());

                Some((name, Value::String(c.as_str().to_owned())))
            })
            .collect();
        Ok(ResolveResult::Owned(Value::Object(v)))
    }
}

regex_function!(RegexReplaceFunction, "regex_replace", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for RegexReplaceFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let arg = self.args[0].resolve(state)?;
        let arg = arg.try_as_string(Self::INFO.name, &self.span)?;
        let repl = self.args[1].resolve(state)?;
        let repl = repl.try_as_string(Self::INFO.name, &self.span)?;

        let r = self.re.replace(arg.as_ref(), repl.as_ref()).into_owned();
        Ok(ResolveResult::Owned(Value::String(r)))
    }
}

regex_function!(RegexReplaceAllFunction, "regex_replace_all", 2);

impl<'a: 'c, 'c> Expression<'a, 'c> for RegexReplaceAllFunction {
    fn resolve(
        &'a self,
        state: &mut crate::expressions::ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let arg = self.args[0].resolve(state)?;
        let arg = arg.try_as_string(Self::INFO.name, &self.span)?;
        let repl = self.args[1].resolve(state)?;
        let repl = repl.try_as_string(Self::INFO.name, &self.span)?;

        let r = self
            .re
            .replace_all(arg.as_ref(), repl.as_ref())
            .into_owned();
        Ok(ResolveResult::Owned(Value::String(r)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::{compile_expression, BuildError, CompileError};

    #[test]
    pub fn test_regex_is_match() {
        let expr = compile_expression(
            r#"
            {
                "v1": regex_is_match("test", ".*"),
                "v2": regex_is_match("test", "^[te]{2}[st]{2}$"),
                "v3": regex_is_match("æøå", "^[æøå]{3}$"),
                "v4": regex_is_match("test", "^not test$")
            }
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let v = res.as_object().unwrap();

        assert_eq!(v["v1"], true);
        assert_eq!(v["v2"], true);
        assert_eq!(v["v3"], true);
        assert_eq!(v["v4"], false);
    }

    #[test]
    pub fn test_regex_first_match() {
        let expr = compile_expression(
            r#"
            {
                "v1": regex_first_match("test", "^te[s]"),
                "v2": regex_first_match("æøå", "^æ[øå]"),
                "v3": regex_first_match("test tets", "te[st]{2}"),
                "v4": regex_first_match("test", "^not test$")
            }
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let v = res.as_object().unwrap();

        assert_eq!(v["v1"], "tes");
        assert_eq!(v["v2"], "æø");
        assert_eq!(v["v3"], "test");
        assert_eq!(v["v4"], Value::Null);
    }

    #[test]
    pub fn test_regex_matches() {
        let expr = compile_expression(
            r#"
            {
                "v1": regex_matches("tets", "t."),
                "v2": regex_matches("æøå", "[æøå]"),
                "v3": regex_matches("test", "^test$"),
                "v4": regex_matches("test", "^not test$")
            }
            "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let v = res.as_object().unwrap();

        assert_eq!(
            v["v1"],
            Value::Array(vec![Value::from("te"), Value::from("ts")])
        );
        assert_eq!(
            v["v2"],
            Value::Array(vec![Value::from("æ"), Value::from("ø"), Value::from("å")])
        );
        assert_eq!(v["v3"], Value::Array(vec![Value::from("test")]));
        assert_eq!(v["v4"], Value::Array(vec![]));
    }

    #[test]
    pub fn test_regex_captures() {
        let expr = compile_expression(
            r#"
            {
                "v1": regex_captures("test", "te([st]{2})"),
                "v2": regex_captures("æøå", "[æøå]{3}"),
                "v3": regex_captures("test string", "^test (?<val>.*)"),
                "v4": regex_captures("test 123 456 789", "^test (?<v1>[0-9]{3}) (?<v2>[0-9]{3}) (?<v3>[0-9]{3})$"),
                "v5": regex_captures("test", "^not (?<v>test)$")
            }
            "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let v = res.as_object().unwrap();

        assert_eq!(
            v["v1"],
            json!({
                "0": "test",
                "1": "st"
            })
        );
        assert_eq!(
            v["v2"],
            json!({
                "0": "æøå",
            })
        );
        assert_eq!(
            v["v3"],
            json!({
                "0": "test string",
                "val": "string"
            })
        );
        assert_eq!(
            v["v4"],
            json!({
                "0": "test 123 456 789",
                "v1": "123",
                "v2": "456",
                "v3": "789"
            })
        );
        assert_eq!(v["v5"], Value::Null);
    }

    #[test]
    pub fn test_regex_replace() {
        let expr = compile_expression(
            r#"
            {
                "v1": "test".regex_replace("t", "s"),
                "v2": regex_replace("æøå", "[æø]{2}", "nope"),
                "v3": regex_replace("test string", "test (?<v>[a-z]*)", "also $v"),
                "v4": regex_replace("test", "^123$", "nope"),
            }
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let v = res.as_object().unwrap();
        assert_eq!(v["v1"], "sest");
        assert_eq!(v["v2"], "nopeå");
        assert_eq!(v["v3"], "also string");
        assert_eq!(v["v4"], "test");
    }

    #[test]
    pub fn test_regex_replace_all() {
        let expr = compile_expression(
            r#"
            {
                "v1": "test".regex_replace_all("t", "s"),
                "v2": "test 123 test 456".regex_replace_all("test (?<v>[0-9]*)", "${v}_key"),
                "v3": regex_replace_all("foo bar baz", "\\s*", ""),
                "v4": regex_replace_all("test", "123", "nope"),
            }
            "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let v = res.as_object().unwrap();
        assert_eq!(v["v1"], "sess");
        assert_eq!(v["v2"], "123_key 456_key");
        assert_eq!(v["v3"], "foobarbaz");
        assert_eq!(v["v4"], "test");
    }

    #[test]
    pub fn test_regex_non_constant() {
        let e =
            compile_expression(r#"regex_is_match("test", concat("te", "st"))"#, &[]).unwrap_err();
        match e {
            CompileError::Build(BuildError::Other(v)) => {
                assert_eq!(v.detail, "Regex must be constant at compile time");
            }
            r => panic!("Unexpected error {r}"),
        }
    }

    #[test]
    pub fn test_regex_non_constant_2() {
        let e = compile_expression(r#"regex_is_match("test", 12345)"#, &[]).unwrap_err();
        match e {
            CompileError::Build(BuildError::Other(v)) => {
                assert_eq!(v.detail, "Regex must be constant at compile time");
            }
            r => panic!("Unexpected error {r}"),
        }
    }

    #[test]
    pub fn test_regex_invalid() {
        let e = compile_expression(r#"regex_is_match("test", "te[[")"#, &[]).unwrap_err();
        match e {
            CompileError::Build(BuildError::Other(v)) => {
                assert!(v.detail.starts_with("Regex compilation failed:"))
            }
            r => panic!("Unexpected error {r}"),
        }
    }
}
