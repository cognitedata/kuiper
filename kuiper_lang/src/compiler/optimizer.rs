use crate::{
    expressions::{Constant, Expression, ExpressionExecutionState, ExpressionMeta, ExpressionType},
    TransformError,
};

fn is_deterministic(expr: &mut ExpressionType) -> bool {
    if !expr.is_deterministic() {
        return false;
    }

    for child in expr.iter_children_mut() {
        if !is_deterministic(child) {
            return false;
        }
    }

    true
}

fn resolve_constants(
    root: &mut ExpressionType,
    num_inputs: usize,
    opcount: &mut i64,
    max_opcount: i64,
) -> Result<Option<ExpressionType>, TransformError> {
    // If there are no children, no further optimization may be done
    if root.iter_children_mut().next().is_none() {
        return Ok(None);
    }

    let data = vec![None; num_inputs];
    let mut state = ExpressionExecutionState::new(&data, opcount, max_opcount);

    let res = match root.resolve(&mut state).map(|r| r.into_owned()) {
        // If resolution succeeds, we can replace this operator with a constant
        Ok(x) if is_deterministic(root) => Ok(Some(ExpressionType::Constant(Constant::new(x)))),
        Ok(_) => Ok(None),
        Err(e) => match e {
            // Any error that is not a source missing error would be a bug in this position,
            // since any execution that is variable between runs would return a source missing error before anything else.
            TransformError::SourceMissingError(_) => {
                // If the source is missing we should try to optimize each child.
                for child in root.iter_children_mut() {
                    let res: Option<ExpressionType> =
                        resolve_constants(child, num_inputs, opcount, max_opcount)?;
                    if let Some(res) = res {
                        *child = res;
                    }
                }
                Ok(None)
            }
            _ => Err(e),
        },
    };
    res
}

/// Run the optimizer. For now this only catches a few consistency errors and resolves any constant expressions.
pub fn optimize(
    mut root: ExpressionType,
    num_inputs: usize,
    max_opcount: i64,
) -> Result<ExpressionType, TransformError> {
    let mut opcount = 0;

    let res = resolve_constants(&mut root, num_inputs, &mut opcount, max_opcount)?;
    match res {
        Some(x) => Ok(x),
        None => Ok(root),
    }
}

#[cfg(test)]
mod tests {
    use logos::Span;

    use crate::{
        compiler::exec_tree::ExecTreeBuilder, expressions::ExpressionType, lexer::Lexer,
        parse::ProgramParser, CompileError, TransformError,
    };

    use super::optimize;

    fn parse(inp: &str, inputs: &[&str]) -> Result<ExpressionType, CompileError> {
        let lex = Lexer::new(inp);
        let parser = ProgramParser::new();
        let res = parser.parse(lex)?;
        let res = ExecTreeBuilder::new(res, inputs, &Default::default())?.build()?;
        let res = optimize(res, inputs.len(), 100_000)?;
        Ok(res)
    }

    fn parse_fail_optimizer(inp: &str, inputs: &[&str]) -> TransformError {
        match parse(inp, inputs) {
            Ok(_) => panic!("Expected parse + optimize to fail"),
            Err(x) => match x {
                CompileError::Optimizer(e) => e,
                _ => panic!("Got incorrect "),
            },
        }
    }

    #[test]
    pub fn test_constant_math() {
        let expr = parse("2 + 2 * (2 - 2 / 2) + pow(3, 2)", &[]).unwrap();
        assert_eq!("13.0", expr.to_string())
    }

    #[test]
    pub fn test_mixed_expression() {
        let expr = parse("2 + 2 * 3 - input.id", &["input"]).unwrap();
        assert_eq!("(8 - $0.id)", expr.to_string());
    }

    #[test]
    pub fn test_fancy_expression() {
        let expr = parse("2 + if(2 > 1, 3, 'uh oh')", &[]).unwrap();
        assert_eq!("5", expr.to_string());
    }

    #[test]
    pub fn test_cast() {
        let expr = parse("2 + int('-2')", &[]).unwrap();
        assert_eq!("0", expr.to_string());
    }

    #[test]
    pub fn test_incorrect_function_arg() {
        let err = parse_fail_optimizer("2 + pow(3, 'test')", &[]);
        match err {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.desc, "pow. Got string, expected number");
                assert_eq!(d.span, Span { start: 4, end: 18 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_divide_by_zero() {
        let err = parse_fail_optimizer("2 / 0", &[]);
        match err {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 2, end: 3 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_mixed_optimizer_order() {
        let expr = parse(
            "[1, 2, 3].map(a => a + 1)[0] + input + input2 + [1, 2, 3].map(a => a + 2)[1]",
            &["input", "input2"],
        )
        .unwrap();
        assert_eq!("(((2 + $0) + $1) + 4)", expr.to_string());
    }
}
