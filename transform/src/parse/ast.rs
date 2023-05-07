use std::fmt::Display;

use crate::expressions::{Operator, UnaryOperator};

pub enum Selector {
    Expression(Box<Expression>),
    String(String),
}

impl Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Expression(x) => write!(f, "[{x}]"),
            Selector::String(x) => write!(f, ".{x}"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Constant {
    String(String),
    PositiveInteger(u64),
    NegativeInteger(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::String(x) => write!(f, r#""{x}""#),
            Constant::PositiveInteger(x) => write!(f, "{x}"),
            Constant::NegativeInteger(x) => write!(f, "{x}"),
            Constant::Float(x) => write!(f, "{x}"),
            Constant::Bool(x) => write!(f, "{x}"),
            Constant::Null => write!(f, "null"),
        }
    }
}

pub enum FunctionParameter {
    Expression(Expression),
    Lambda {
        args: Vec<String>,
        inner: Expression,
    },
}

impl Display for FunctionParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionParameter::Expression(x) => write!(f, "{x}"),
            FunctionParameter::Lambda { args, inner } => {
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

pub struct OpExpression {
    pub lhs: Box<Expression>,
    pub operator: Operator,
    pub rhs: Box<Expression>,
}

pub enum Expression {
    BinaryOperation(OpExpression),
    UnaryOperation {
        operator: UnaryOperator,
        rhs: Box<Expression>,
    },
    Array(Vec<Expression>),
    Object(Vec<(Expression, Expression)>),
    Selector {
        lhs: Box<Expression>,
        sel: Selector,
    },
    Constant(Constant),
    Function {
        name: String,
        args: Vec<FunctionParameter>,
    },
    Variable(String),
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::BinaryOperation(OpExpression { lhs, operator, rhs }) => {
                write!(f, "({lhs} {operator} {rhs})")
            }
            Expression::UnaryOperation { operator, rhs } => write!(f, "{operator}{rhs}"),
            Expression::Array(a) => {
                write!(f, "[")?;
                let mut needs_comma = false;
                for arg in a {
                    if needs_comma {
                        write!(f, ", ")?;
                    }
                    needs_comma = true;
                    write!(f, "{arg}")?;
                }
                write!(f, "]")?;
                Ok(())
            }
            Expression::Object(a) => {
                write!(f, "{{")?;
                let mut needs_comma = false;
                for (lh, rh) in a {
                    if needs_comma {
                        write!(f, ", ")?;
                    }
                    needs_comma = true;
                    write!(f, "{lh}: {rh}")?;
                }
                write!(f, "}}")
            }
            Expression::Selector { lhs, sel } => write!(f, "{lhs}{sel}"),
            Expression::Constant(c) => write!(f, "{c}"),
            Expression::Function { name, args } => {
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
            Expression::Variable(v) => write!(f, "{v}"),
        }
    }
}
