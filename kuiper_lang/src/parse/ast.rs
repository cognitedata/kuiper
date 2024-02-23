use std::fmt::Display;

use logos::Span;
use serde_json::{Number, Value};

use crate::expressions::{Operator, TypeLiteral, UnaryOperator};

#[derive(Debug)]
pub enum Selector {
    Expression(Box<Expression>),
    String(String, Span),
}

impl Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Expression(x) => write!(f, "[{x}]"),
            Selector::String(x, _) => write!(f, ".{x}"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Constant {
    String(String),
    Integer(u64),
    Float(f64),
    Bool(bool),
    Null,
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::String(x) => write!(f, r#""{x}""#),
            Constant::Integer(x) => write!(f, "{x}"),
            Constant::Float(x) => write!(f, "{x}"),
            Constant::Bool(x) => write!(f, "{x}"),
            Constant::Null => write!(f, "null"),
        }
    }
}

impl From<Constant> for Value {
    fn from(val: Constant) -> Self {
        match val {
            Constant::String(s) => Value::String(s),
            Constant::Integer(x) => Value::Number(x.into()),
            Constant::Float(x) => {
                Value::Number(Number::from_f64(x).unwrap_or_else(|| Number::from_f64(0.0).unwrap()))
            }
            Constant::Bool(x) => Value::Bool(x),
            Constant::Null => Value::Null,
        }
    }
}

#[derive(Debug)]
pub enum FunctionParameter {
    Expression(Expression),
    Lambda {
        args: Vec<String>,
        inner: Expression,
        loc: Span,
    },
}

impl Display for FunctionParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionParameter::Expression(x) => write!(f, "{x}"),
            FunctionParameter::Lambda {
                args,
                inner,
                loc: _,
            } => {
                write!(f, "(")?;
                let mut needs_comma = false;
                for arg in args {
                    if needs_comma {
                        write!(f, ", ")?;
                    }
                    needs_comma = true;
                    write!(f, "{arg}")?;
                }
                write!(f, ") => ")?;
                write!(f, "{inner}")
            }
        }
    }
}

#[derive(Debug)]
pub struct OpExpression {
    pub lhs: Box<Expression>,
    pub operator: Operator,
    pub rhs: Box<Expression>,
}

#[derive(Debug)]
pub struct IsExpression {
    pub lhs: Box<Expression>,
    pub rhs: TypeLiteral,
    pub not: bool,
}

#[derive(Debug)]
pub enum ArrayElementAst {
    Expression(Expression),
    Concat(Expression),
}

#[derive(Debug)]
pub enum ObjectElementAst {
    Pair(Expression, Expression),
    Concat(Expression),
}

#[derive(Debug)]
pub enum Expression {
    BinaryOperation(OpExpression, Span),
    Is(IsExpression),
    UnaryOperation {
        operator: UnaryOperator,
        rhs: Box<Expression>,
        loc: Span,
    },
    Array(Vec<ArrayElementAst>, Span),
    Object(Vec<ObjectElementAst>, Span),
    Selector {
        lhs: Box<Expression>,
        sel: Selector,
        loc: Span,
    },
    Constant(Constant, Span),
    Function {
        name: String,
        args: Vec<FunctionParameter>,
        loc: Span,
    },
    Variable(String, Span),
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::BinaryOperation(OpExpression { lhs, operator, rhs }, _) => {
                write!(f, "({lhs} {operator} {rhs})")
            }
            Expression::UnaryOperation {
                operator,
                rhs,
                loc: _,
            } => write!(f, "{operator}{rhs}"),
            Expression::Array(a, _) => {
                write!(f, "[")?;
                let mut needs_comma = false;
                for arg in a {
                    if needs_comma {
                        write!(f, ", ")?;
                    }
                    needs_comma = true;
                    match arg {
                        ArrayElementAst::Expression(x) => write!(f, "{x}")?,
                        ArrayElementAst::Concat(x) => write!(f, "..{x}")?,
                    }
                }
                write!(f, "]")?;
                Ok(())
            }
            Expression::Object(a, _) => {
                write!(f, "{{")?;
                let mut needs_comma = false;
                for k in a {
                    if needs_comma {
                        write!(f, ", ")?;
                    }
                    needs_comma = true;
                    match k {
                        ObjectElementAst::Pair(lh, rh) => write!(f, "{lh}: {rh}")?,
                        ObjectElementAst::Concat(x) => write!(f, "..{x}")?,
                    }
                }
                write!(f, "}}")
            }
            Expression::Selector { lhs, sel, loc: _ } => write!(f, "{lhs}{sel}"),
            Expression::Constant(c, _) => write!(f, "{c}"),
            Expression::Function { name, args, loc: _ } => {
                write!(f, "{name}(")?;
                let mut needs_comma = false;
                for arg in args {
                    if needs_comma {
                        write!(f, ", ")?;
                    }
                    needs_comma = true;
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }
            Expression::Variable(v, _) => write!(f, "{v}"),
            Expression::Is(i) => write!(f, "({} is {})", i.lhs, i.rhs),
        }
    }
}
