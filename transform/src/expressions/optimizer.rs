use std::collections::HashMap;

use crate::TransformError;

use super::{base::ExpressionMeta, Constant, Expression, ExpressionExecutionState, ExpressionType};

fn resolve_constants(
    root: &mut ExpressionType,
    known_inputs: &mut HashMap<String, usize>,
    empty_state: &ExpressionExecutionState,
) -> Result<Option<ExpressionType>, TransformError> {
    if let ExpressionType::Selector(s) = root {
        s.resolve_first_item(empty_state, known_inputs)?;
    }

    // If there are no children, no further optimization may be done
    if root.num_children() == 0 {
        return Ok(None);
    }

    let mut temp_inputs: Option<Vec<String>> = None;
    if let ExpressionType::Lambda(l) = root {
        let mut max_inputs = known_inputs.values().max().copied().unwrap_or_default();
        let inputs = l.input_names.clone();
        for inp in &inputs {
            if known_inputs.contains_key(inp) {
                return Err(TransformError::new_invalid_operation(
                    format!("Function input {inp} is already defined"),
                    &l.span,
                    empty_state.id,
                ));
            }
            max_inputs += 1;
            known_inputs.insert(inp.clone(), max_inputs);
        }
        temp_inputs = Some(inputs);
    }

    let res = match root.resolve(empty_state) {
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
                    let res = resolve_constants(
                        root.get_child_mut(idx).unwrap(),
                        known_inputs,
                        empty_state,
                    )?;
                    if let Some(res) = res {
                        root.set_child(idx, res);
                    }
                }
                Ok(None)
            }
            _ => Err(e),
        },
    };

    if let Some(inputs) = temp_inputs {
        for inp in &inputs {
            known_inputs.remove(inp);
        }
    }
    res
}

/// Run the optimizer. For now this only catches a few consistency errors and resolves any constant expressions.
pub fn optimize(
    mut root: ExpressionType,
    known_inputs: &mut HashMap<String, usize>,
) -> Result<ExpressionType, TransformError> {
    let data = Vec::new();
    let empty_state = ExpressionExecutionState::new(&data, "optimizer");

    let res = resolve_constants(&mut root, known_inputs, &empty_state)?;
    match res {
        Some(x) => Ok(x),
        None => Ok(root),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use logos::{Logos, Span};

    use crate::{expressions::ExpressionType, lexer::Token, CompileError, Parser, TransformError};

    use super::optimize;

    fn parse(inp: &str, inputs: &[&str]) -> Result<ExpressionType, CompileError> {
        let lex = Token::lexer(inp);
        let res = Parser::new(lex)
            .parse()
            .map_err(|e| CompileError::from_parser_err(e, "test", None))?;
        let mut input_map = HashMap::new();
        for (idx, input) in inputs.iter().enumerate() {
            input_map.insert(input.to_string(), idx);
        }
        let res = optimize(res, &mut input_map)
            .map_err(|e| CompileError::optimizer_err(e, "test", None))?;
        Ok(res)
    }

    fn parse_fail_optimizer(inp: &str, inputs: &[&str]) -> TransformError {
        match parse(inp, inputs) {
            Ok(_) => panic!("Expected parse + optimize to fail"),
            Err(x) => match x {
                CompileError::Optimizer(e) => e.err,
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
        let expr = parse("2 + 2 * 3 - $input.id", &["input"]).unwrap();
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
                assert_eq!(d.id, "optimizer");
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
                assert_eq!(d.id, "optimizer");
                assert_eq!(d.desc, "Divide by zero");
                assert_eq!(d.span, Span { start: 2, end: 3 });
            }
            _ => panic!("Wrong type of error {err:?}"),
        }
    }
}
