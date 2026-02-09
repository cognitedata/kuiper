use std::{collections::HashMap, fmt::Display};

use logos::Span;
use thiserror::Error;

use crate::{
    expressions::{
        get_function_expression, ArrayElement, ArrayExpression, ExpressionType, IfExpression,
        IsExpression, LambdaExpression, MacroCallExpression, ObjectElement, ObjectExpression,
        OpExpression, SelectorElement, SelectorExpression, SourceElement, UnaryOpExpression,
    },
    parse::{Expression, FunctionParameter, Lambda, Macro, Program, Selector},
};

use super::CompilerConfig;

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
    /// Incorrect number of function arguments.
    #[error("{0}")]
    NFunctionArgs(CompileErrorData),
    /// A lambda was encountered where an expression was expected.
    #[error("{0}")]
    UnexpectedLambda(CompileErrorData),
    /// An unrecognized function was called.
    #[error("Unrecognized function: {0}")]
    UnrecognizedFunction(CompileErrorData),
    /// An unknown variable was referenced.
    #[error("Unknown variable: {0}")]
    UnknownVariable(CompileErrorData),
    /// A variable was defined more than once.
    #[error("Variable already defined: {0}")]
    VariableConflict(CompileErrorData),
    /// Some other compile error.
    #[error("{0}")]
    Other(CompileErrorData),
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
    pub(crate) fn other(position: Span, err: &str) -> Self {
        Self::Other(CompileErrorData {
            position,
            detail: err.to_string(),
        })
    }
}

pub(crate) struct ExecTreeBuilder {
    inner: BuilderInner,
    expression: Expression,
}

struct MacroCounter {
    num_expansions: i32,
    max_expansions: i32,
}

impl MacroCounter {
    pub fn new(max_expansions: i32) -> Self {
        Self {
            max_expansions,
            num_expansions: 0,
        }
    }

    pub fn expand_macro(&mut self, loc: Span) -> Result<(), BuildError> {
        if self.max_expansions >= 0 && self.max_expansions <= self.num_expansions {
            return Err(BuildError::other(
                loc,
                &format!(
                    "Too many macro expansions, maximum is {}",
                    self.max_expansions
                ),
            ));
        }
        self.num_expansions += 1;
        Ok(())
    }
}

struct BuilderInner {
    known_inputs: HashMap<String, usize>,
    macros: HashMap<String, Macro>,
    macro_counter: MacroCounter,
    macro_stack: Vec<String>,
}

impl ExecTreeBuilder {
    pub fn new(
        program: Program,
        known_inputs: &[&str],
        compiler_config: &CompilerConfig,
    ) -> Result<Self, BuildError> {
        let mut inputs = HashMap::new();
        for inp in known_inputs {
            inputs.insert((*inp).to_owned(), inputs.len());
        }
        let mut macros = HashMap::new();
        for mc in program.macros {
            let span = mc.body.loc.clone();
            if macros.insert(mc.name.clone(), mc).is_some() {
                return Err(BuildError::other(span, "Duplicate macro definition"));
            }
        }
        Ok(Self {
            inner: BuilderInner {
                known_inputs: inputs,
                macros,
                macro_counter: MacroCounter::new(compiler_config.max_macro_expansions),
                macro_stack: Vec::new(),
            },
            expression: program.expression,
        })
    }

    pub fn build(mut self) -> Result<ExpressionType, BuildError> {
        self.inner.build_expression(self.expression, 0)
    }
}

impl BuilderInner {
    fn build_selector(
        &mut self,
        lhs: Expression,
        sel: Selector,
        depth: usize,
    ) -> Result<Vec<SelectorElement>, BuildError> {
        let x = match sel {
            Selector::Expression(x) => {
                SelectorElement::Expression(Box::new(self.build_expression(*x, depth)?))
            }
            Selector::String(x, s) => SelectorElement::Constant(x, s),
        };

        match lhs {
            Expression::Selector { lhs, sel, loc: _ } => {
                let mut ch = self.build_selector(*lhs, sel, depth)?;
                ch.push(x);
                Ok(ch)
            }
            Expression::Variable(v, s) => Ok(vec![SelectorElement::Constant(v, s), x]),
            r => Ok(vec![
                SelectorElement::Expression(Box::new(self.build_expression(r, depth)?)),
                x,
            ]),
        }
    }

    fn build_lambda(&mut self, expr: Lambda, depth: usize) -> Result<ExpressionType, BuildError> {
        let Lambda { args, inner, loc } = expr;
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
        let r = LambdaExpression::new(args, self.build_expression(inner, depth)?, loc)?;
        for var in temp_variables {
            self.known_inputs.remove(&var);
        }
        Ok(ExpressionType::Lambda(r))
    }

    fn build_function_param(
        &mut self,
        expr: FunctionParameter,
        depth: usize,
    ) -> Result<ExpressionType, BuildError> {
        match expr {
            FunctionParameter::Expression(x) => self.build_expression(x, depth),
            FunctionParameter::Lambda(l) => self.build_lambda(l, depth),
        }
    }

    fn build_macro_call(
        &mut self,
        mac: Macro,
        args: Vec<FunctionParameter>,
        span: Span,
        depth: usize,
    ) -> Result<ExpressionType, BuildError> {
        self.macro_counter.expand_macro(span.clone())?;
        if mac.body.args.len() != args.len() {
            return Err(BuildError::n_function_args(
                span,
                &format!("Expected {} arguments to macro", mac.body.args.len()),
            ));
        }
        if self.macro_stack.contains(&mac.name) {
            return Err(BuildError::other(
                span,
                "Recursive macro calls are not allowed",
            ));
        }
        let mut built_args = Vec::new();
        for arg in args {
            let expr = match arg {
                FunctionParameter::Expression(e) => e,
                FunctionParameter::Lambda(e) => return Err(BuildError::unexpected_lambda(&e.loc)),
            };
            built_args.push(self.build_expression(expr, depth)?);
        }
        self.macro_stack.push(mac.name.clone());
        let inner = self.build_lambda(mac.body, depth)?;
        self.macro_stack.pop();
        Ok(ExpressionType::MacroCallExpression(
            MacroCallExpression::new(inner, built_args, span)?,
        ))
    }

    fn resolve_input(&self, source: &str, span: Span) -> Result<SourceElement, BuildError> {
        if let Some(idx) = self.known_inputs.get(source) {
            Ok(SourceElement::CompiledInput(*idx))
        } else {
            Err(BuildError::unknown_variable(span, source))
        }
    }

    pub fn build_expression(
        &mut self,
        ast: Expression,
        depth: usize,
    ) -> Result<ExpressionType, BuildError> {
        // Setting this too high might result in a stack overflow.
        if depth > 200 {
            return Err(BuildError::other(
                Span { start: 0, end: 0 },
                "Recursion depth limit exceeded during compilation",
            ));
        }
        match ast {
            Expression::BinaryOperation(b, span) => {
                Ok(ExpressionType::Operator(OpExpression::new(
                    b.operator,
                    self.build_expression(*b.lhs, depth + 1)?,
                    self.build_expression(*b.rhs, depth + 1)?,
                    span,
                )?))
            }
            Expression::UnaryOperation { operator, rhs, loc } => Ok(ExpressionType::UnaryOperator(
                UnaryOpExpression::new(operator, self.build_expression(*rhs, depth + 1)?, loc)?,
            )),
            Expression::Array(arr, span) => Ok(ExpressionType::Array(ArrayExpression::new(
                arr.into_iter()
                    .map(|e| match e {
                        crate::parse::ArrayElementAst::Expression(x) => Ok(
                            ArrayElement::Expression(self.build_expression(x, depth + 1)?),
                        ),
                        crate::parse::ArrayElementAst::Concat(x) => {
                            Ok(ArrayElement::Concat(self.build_expression(x, depth + 1)?))
                        }
                    })
                    .collect::<Result<Vec<ArrayElement>, _>>()?,
                span,
            )?)),
            Expression::Object(it, span) => Ok(ExpressionType::Object(ObjectExpression::new(
                it.into_iter()
                    .map::<Result<ObjectElement, BuildError>, _>(|e| match e {
                        crate::parse::ObjectElementAst::Pair(k, v) => Ok(ObjectElement::Pair(
                            self.build_expression(k, depth + 1)?,
                            self.build_expression(v, depth + 1)?,
                        )),
                        crate::parse::ObjectElementAst::Concat(x) => {
                            Ok(ObjectElement::Concat(self.build_expression(x, depth + 1)?))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                span,
            )?)),
            Expression::Selector { lhs, sel, loc } => {
                let elems = self.build_selector(*lhs, sel, depth + 1)?;
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
            Expression::Function { name, args, loc } => {
                if let Some(m) = self.macros.get(&name).cloned() {
                    self.build_macro_call(m, args, loc, depth + 1)
                } else {
                    get_function_expression(
                        loc,
                        &name,
                        args.into_iter()
                            .map(|e| self.build_function_param(e, depth + 1))
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                }
            }
            Expression::Variable(v, span) => Ok(ExpressionType::Selector(SelectorExpression::new(
                self.resolve_input(&v, span.clone())?,
                vec![],
                span,
            )?)),
            Expression::Is(i) => Ok(ExpressionType::Is(IsExpression::new(
                self.build_expression(*i.lhs, depth + 1)?,
                i.rhs,
                i.not,
            )?)),
            Expression::If { args, loc } => Ok(ExpressionType::If(IfExpression::new(
                args.into_iter()
                    .map(|e| self.build_expression(e, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?,
                loc,
            ))),
        }
    }
}
