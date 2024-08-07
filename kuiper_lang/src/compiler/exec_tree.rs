use std::{collections::HashMap, fmt::Display};

use logos::Span;
use thiserror::Error;

use crate::{
    expressions::{
        get_function_expression, ArrayElement, ArrayExpression, ExpressionType, IfExpression,
        IsExpression, LambdaExpression, ObjectElement, ObjectExpression, OpExpression,
        SelectorElement, SelectorExpression, SourceElement, UnaryOpExpression,
    },
    parse::{Expression, FunctionParameter, Selector},
};

#[derive(Debug, Error)]
pub struct CompileErrorData {
    pub position: Span,
    pub detail: String,
}

impl Display for CompileErrorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.detail, self.position.start, self.position.end
        )
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
    #[error("Unrecognized function: {0}")]
    UnrecognizedFunction(CompileErrorData),
    #[error("Unknown variable: {0}")]
    UnknownVariable(CompileErrorData),
    #[error("Variable already defined: {0}")]
    VariableConflict(CompileErrorData),
}

impl BuildError {
    pub(crate) fn n_function_args(position: Span, detail: &str) -> Self {
        Self::NFunctionArgs(CompileErrorData {
            position,
            detail: format!("Incorrect number of function args: {detail}"),
        })
    }
    pub(crate) fn unexpected_lambda(position: &Span) -> Self {
        Self::UnexpectedLambda(CompileErrorData {
            position: position.clone(),
            detail: "Expected expression, got lambda".to_owned(),
        })
    }
    pub(crate) fn unrecognized_function(position: Span, symbol: &str) -> Self {
        Self::UnrecognizedFunction(CompileErrorData {
            position,
            detail: symbol.to_string(),
        })
    }
    pub(crate) fn unknown_variable(position: Span, var: &str) -> Self {
        Self::UnknownVariable(CompileErrorData {
            position,
            detail: var.to_string(),
        })
    }
    pub(crate) fn variable_conflict(position: Span, var: &str) -> Self {
        Self::VariableConflict(CompileErrorData {
            position,
            detail: var.to_string(),
        })
    }
}

pub(crate) struct ExecTreeBuilder {
    inner: BuilderInner,
    expression: Expression,
}

struct BuilderInner {
    known_inputs: HashMap<String, usize>,
}

impl ExecTreeBuilder {
    pub fn new(expr: Expression, known_inputs: &[&str]) -> Self {
        let mut inputs = HashMap::new();
        for inp in known_inputs {
            inputs.insert((*inp).to_owned(), inputs.len());
        }
        Self {
            inner: BuilderInner {
                known_inputs: inputs,
            },
            expression: expr,
        }
    }

    pub fn build(mut self) -> Result<ExpressionType, BuildError> {
        self.inner.build_expression(self.expression)
    }
}

impl BuilderInner {
    fn build_selector(
        &mut self,
        lhs: Expression,
        sel: Selector,
    ) -> Result<Vec<SelectorElement>, BuildError> {
        let x = match sel {
            Selector::Expression(x) => {
                SelectorElement::Expression(Box::new(self.build_expression(*x)?))
            }
            Selector::String(x, s) => SelectorElement::Constant(x, s),
        };

        match lhs {
            Expression::Selector { lhs, sel, loc: _ } => {
                let mut ch = self.build_selector(*lhs, sel)?;
                ch.push(x);
                Ok(ch)
            }
            Expression::Variable(v, s) => Ok(vec![SelectorElement::Constant(v, s), x]),
            r => Ok(vec![
                SelectorElement::Expression(Box::new(self.build_expression(r)?)),
                x,
            ]),
        }
    }

    fn build_function_param(
        &mut self,
        expr: FunctionParameter,
    ) -> Result<ExpressionType, BuildError> {
        match expr {
            FunctionParameter::Expression(x) => Ok(self.build_expression(x)?),
            FunctionParameter::Lambda { args, inner, loc } => {
                // Temporarily add lambda arguments as variables.
                let mut temp_variables = vec![];
                for inp in args.iter() {
                    temp_variables.push(inp.clone());
                    if self
                        .known_inputs
                        .insert(inp.clone(), self.known_inputs.len())
                        .is_some()
                    {
                        return Err(BuildError::variable_conflict(loc, inp));
                    }
                }
                let r = LambdaExpression::new(args, self.build_expression(inner)?, loc)?;
                for var in temp_variables {
                    self.known_inputs.remove(&var);
                }
                Ok(ExpressionType::Lambda(r))
            }
        }
    }

    fn resolve_input(&self, source: &str, span: Span) -> Result<SourceElement, BuildError> {
        if let Some(idx) = self.known_inputs.get(source) {
            Ok(SourceElement::CompiledInput(*idx))
        } else {
            Err(BuildError::unknown_variable(span, source))
        }
    }

    pub fn build_expression(&mut self, ast: Expression) -> Result<ExpressionType, BuildError> {
        match ast {
            Expression::BinaryOperation(b, span) => {
                Ok(ExpressionType::Operator(OpExpression::new(
                    b.operator,
                    self.build_expression(*b.lhs)?,
                    self.build_expression(*b.rhs)?,
                    span,
                )?))
            }
            Expression::UnaryOperation { operator, rhs, loc } => Ok(ExpressionType::UnaryOperator(
                UnaryOpExpression::new(operator, self.build_expression(*rhs)?, loc)?,
            )),
            Expression::Array(arr, span) => Ok(ExpressionType::Array(ArrayExpression::new(
                arr.into_iter()
                    .map(|e| match e {
                        crate::parse::ArrayElementAst::Expression(x) => {
                            Ok(ArrayElement::Expression(self.build_expression(x)?))
                        }
                        crate::parse::ArrayElementAst::Concat(x) => {
                            Ok(ArrayElement::Concat(self.build_expression(x)?))
                        }
                    })
                    .collect::<Result<Vec<ArrayElement>, _>>()?,
                span,
            )?)),
            Expression::Object(it, span) => Ok(ExpressionType::Object(ObjectExpression::new(
                it.into_iter()
                    .map::<Result<ObjectElement, BuildError>, _>(|e| match e {
                        crate::parse::ObjectElementAst::Pair(k, v) => Ok(ObjectElement::Pair(
                            self.build_expression(k)?,
                            self.build_expression(v)?,
                        )),
                        crate::parse::ObjectElementAst::Concat(x) => {
                            Ok(ObjectElement::Concat(self.build_expression(x)?))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                span,
            )?)),
            Expression::Selector { lhs, sel, loc } => {
                let elems = self.build_selector(*lhs, sel)?;
                let mut iter = elems.into_iter();
                let first = iter.next().unwrap();

                let source = match first {
                    SelectorElement::Constant(x, s) => self.resolve_input(&x, s)?,
                    SelectorElement::Expression(e) => SourceElement::Expression(e),
                };

                Ok(ExpressionType::Selector(SelectorExpression::new(
                    source,
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
                    .map(|e| self.build_function_param(e))
                    .collect::<Result<Vec<_>, _>>()?,
            )?),
            Expression::Variable(v, span) => Ok(ExpressionType::Selector(SelectorExpression::new(
                self.resolve_input(&v, span.clone())?,
                vec![],
                span,
            )?)),
            Expression::Is(i) => Ok(ExpressionType::Is(IsExpression::new(
                self.build_expression(*i.lhs)?,
                i.rhs,
                i.not,
            )?)),
            Expression::If { args, loc } => Ok(ExpressionType::If(IfExpression::new(
                args.into_iter()
                    .map(|e| self.build_expression(e))
                    .collect::<Result<Vec<_>, _>>()?,
                loc,
            ))),
        }
    }
}
