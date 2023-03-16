use std::collections::HashMap;

use crate::TransformError;

use super::{base::ExpressionMeta, Constant, Expression, ExpressionExecutionState, ExpressionType};

fn resolve_constants(
    root: &mut ExpressionType,
    empty_state: &ExpressionExecutionState,
) -> Result<Option<ExpressionType>, TransformError> {
    // If there are no children, no further optimization may be done
    if root.num_children() == 0 {
        return Ok(None);
    }
    match root.resolve(empty_state) {
        // If resolution succeeds, we can replace this operator with a constant
        Ok(x) => Ok(Some(ExpressionType::Constant(Constant::new(
            x.into_owned(),
        )))),
        Err(e) => match e {
            // Any error that is not a source missing error would be a bug in this position,
            // since any execution that is variable between runs would return a source missing error before anything else.
            TransformError::SourceMissingError(_) => {
                // If the source is missing we should try to optimize each child.
                for idx in 0..root.num_children() {
                    let res = resolve_constants(root.get_child_mut(idx).unwrap(), empty_state)?;
                    if let Some(res) = res {
                        root.set_child(idx, res);
                    }
                }
                Ok(None)
            }
            _ => Err(e),
        },
    }
}

/// Run the optimizer. For now this only catches a few consistency errors and resolves any constant expressions.
pub fn optimize(mut root: ExpressionType) -> Result<ExpressionType, TransformError> {
    let data = Vec::new();
    let map = HashMap::new();
    let empty_state = ExpressionExecutionState::new(&data, &map, "optimizer", 0);

    let res = resolve_constants(&mut root, &empty_state)?;
    match res {
        Some(x) => Ok(x),
        None => Ok(root),
    }
}

#[cfg(test)]
mod tests {
    use logos::{Logos, Span};

    use crate::{expressions::ExpressionType, lexer::Token, CompileError, Parser, TransformError};

    use super::optimize;

    fn parse(inp: &str) -> Result<ExpressionType, CompileError> {
        let lex = Token::lexer(inp);
        let res = Parser::new(lex)
            .parse()
            .map_err(|e| CompileError::from_parser_err(e, "test", None))?;
        let res = optimize(res).map_err(|e| CompileError::optimizer_err(e, "test", None))?;
        Ok(res)
    }

    fn parse_fail_optimizer(inp: &str) -> TransformError {
        match parse(inp) {
            Ok(_) => panic!("Expected parse + optimize to fail"),
            Err(x) => match x {
                CompileError::Optimizer(e) => e.err,
                _ => panic!("Got incorrect "),
            },
        }
    }

    #[test]
    pub fn test_constant_math() {
        let expr = parse("2 + 2 * (2 - 2 / 2) + pow(3, 2)").unwrap();
        assert_eq!("13.0", expr.to_string())
    }

    #[test]
    pub fn test_mixed_expression() {
        let expr = parse("2 + 2 * 3 - $input.id").unwrap();
        assert_eq!("(8 - $input.id)", expr.to_string());
    }

    #[test]
    pub fn test_fancy_expression() {
        let expr = parse("2 + if(2 > 1, 3, 'uh oh')").unwrap();
        assert_eq!("5", expr.to_string());
    }

    #[test]
    pub fn test_cast() {
        let expr = parse("2 + int('-2')").unwrap();
        assert_eq!("0", expr.to_string());
    }

    #[test]
    pub fn test_incorrect_function_arg() {
        let err = parse_fail_optimizer("2 + pow(3, 'test')");
        match err {
            TransformError::IncorrectTypeInField(d) => {
                assert_eq!(d.id, "optimizer");
                assert_eq!(d.desc, "pow. Got string, expected number");
                assert_eq!(d.span, Span { start: 4, end: 18 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }

    #[test]
    pub fn test_divide_by_zero() {
        let err = parse_fail_optimizer("2 / 0");
        match err {
            TransformError::InvalidOperation(d) => {
                assert_eq!(d.id, "optimizer");
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 2, end: 3 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }
}
