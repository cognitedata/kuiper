use std::fmt::Display;

use logos::Span;
use thiserror::Error;

use crate::{
    expressions::{
        get_function_expression, ArrayExpression, ExpressionType, LambdaExpression,
        ObjectExpression, OpExpression, SelectorElement, SelectorExpression, SourceElement,
        UnaryOpExpression,
    },
    parse::{Expression, FunctionParameter, Selector},
};

#[derive(Debug, Error)]
pub struct CompileErrorData {
    pub position: Span,
    pub detail: Option<String>,
}

impl Display for CompileErrorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(e) = &self.detail {
            write!(f, "{} at {}..{}", e, self.position.start, self.position.end)
        } else {
            write!(f, "at {}..{}", self.position.start, self.position.end)
        }
    }
}

/// Error returned from the stage converting the AST to an executable expression.
/// This is typically a missing function, or the wrong number or type of function arguments.
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("{0}")]
    NFunctionArgs(CompileErrorData),
    #[error("{0}")]
    UnexpectedLambda(CompileErrorData),
    #[error("{0}")]
    UnrecognizedFunction(CompileErrorData),
}

impl BuildError {
    pub(crate) fn n_function_args(position: Span, detail: &str) -> Self {
        Self::NFunctionArgs(CompileErrorData {
            position,
            detail: Some(format!("Incorrect number of function args: {detail}")),
        })
    }
    pub(crate) fn unexpected_lambda(position: &Span) -> Self {
        Self::UnexpectedLambda(CompileErrorData {
            position: position.clone(),
            detail: None,
        })
    }
    pub(crate) fn unrecognized_function(position: Span, symbol: &str) -> Self {
        Self::UnrecognizedFunction(CompileErrorData {
            position,
            detail: Some(format!("Unrecognized function: {symbol}")),
        })
    }
}

fn build_selector(lhs: Expression, sel: Selector) -> Result<Vec<SelectorElement>, BuildError> {
    let x = match sel {
        Selector::Expression(x) => SelectorElement::Expression(Box::new(from_ast(*x)?)),
        Selector::String(x) => SelectorElement::Constant(x),
    };

    match lhs {
        Expression::Selector { lhs, sel, loc: _ } => {
            let mut ch = build_selector(*lhs, sel)?;
            ch.push(x);
            Ok(ch)
        }
        Expression::Variable(v, _) => Ok(vec![SelectorElement::Constant(v), x]),
        r => Ok(vec![SelectorElement::Expression(Box::new(from_ast(r)?)), x]),
    }
}

fn from_function_param(expr: FunctionParameter) -> Result<ExpressionType, BuildError> {
    match expr {
        FunctionParameter::Expression(x) => Ok(from_ast(x)?),
        FunctionParameter::Lambda { args, inner, loc } => Ok(ExpressionType::Lambda(
            LambdaExpression::new(args, from_ast(inner)?, loc)?,
        )),
    }
}

pub fn from_ast(ast: Expression) -> Result<ExpressionType, BuildError> {
    match ast {
        Expression::BinaryOperation(b, span) => Ok(ExpressionType::Operator(OpExpression::new(
            b.operator,
            from_ast(*b.lhs)?,
            from_ast(*b.rhs)?,
            span,
        )?)),
        Expression::UnaryOperation { operator, rhs, loc } => Ok(ExpressionType::UnaryOperator(
            UnaryOpExpression::new(operator, from_ast(*rhs)?, loc)?,
        )),
        Expression::Array(arr, span) => Ok(ExpressionType::Array(ArrayExpression::new(
            arr.into_iter()
                .map(from_ast)
                .collect::<Result<Vec<_>, _>>()?,
            span,
        )?)),
        Expression::Object(it, span) => Ok(ExpressionType::Object(ObjectExpression::new(
            it.into_iter()
                .map::<Result<(ExpressionType, ExpressionType), BuildError>, _>(|(v1, v2)| {
                    Ok((from_ast(v1)?, from_ast(v2)?))
                })
                .collect::<Result<Vec<_>, _>>()?,
            span,
        )?)),
        Expression::Selector { lhs, sel, loc } => {
            let elems = build_selector(*lhs, sel)?;
            let mut iter = elems.into_iter();
            let first = iter.next().unwrap();
            Ok(ExpressionType::Selector(SelectorExpression::new(
                first.into(),
                iter.collect(),
                loc,
            )?))
        }
        Expression::Constant(c, _span) => Ok(ExpressionType::Constant(
            crate::expressions::Constant::new(c.into()),
        )),
        Expression::Function { name, args, loc } => Ok(get_function_expression(
            loc,
            &name,
            args.into_iter()
                .map(from_function_param)
                .collect::<Result<Vec<_>, _>>()?,
        )?),
        Expression::Variable(v, span) => Ok(ExpressionType::Selector(SelectorExpression::new(
            SourceElement::Input(v),
            vec![],
            span,
        )?)),
    }
}
