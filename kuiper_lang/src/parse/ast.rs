use std::fmt::Display;

use logos::Span;
use serde_json::{Number, Value};

use crate::{
    expressions::{Operator, TypeLiteral, UnaryOperator},
    write_list,
};

#[derive(Debug, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, Clone)]
pub struct Lambda {
    pub args: Vec<String>,
    pub inner: Expression,
    pub loc: Span,
}

impl Display for Lambda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Lambda { args, inner, .. } = &self;
        write!(f, "(")?;
        write_list!(f, args);
        write!(f, ") => ")?;
        write!(f, "{inner}")
    }
}

#[derive(Debug, Clone)]
pub enum FunctionParameter {
    Expression(Expression),
    Lambda(Lambda),
}

impl Display for FunctionParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionParameter::Expression(x) => write!(f, "{x}"),
            FunctionParameter::Lambda(x) => write!(f, "{x}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpExpression {
    pub lhs: Box<Expression>,
    pub operator: Operator,
    pub rhs: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct IsExpression {
    pub lhs: Box<Expression>,
    pub rhs: TypeLiteral,
    pub not: bool,
}

#[derive(Debug, Clone)]
pub enum ArrayElementAst {
    Expression(Expression),
    Concat(Expression),
}

#[derive(Debug, Clone)]
pub enum ObjectElementAst {
    Pair(Expression, Expression),
    Concat(Expression),
}

#[derive(Debug, Clone)]
pub struct Macro {
    pub name: String,
    pub body: Lambda,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub macros: Vec<Macro>,
    pub expression: Expression,
}

#[derive(Debug, Clone)]
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
    If {
        args: Vec<Expression>,
        loc: Span,
    },
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for m in &self.macros {
            write!(f, "{} := {};", m.name, m.body)?;
        }
        write!(f, "{}", self.expression)
    }
}

impl Display for ArrayElementAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expression(x) => write!(f, "{x}"),
            Self::Concat(x) => write!(f, "..{x}"),
        }
    }
}

impl Display for ObjectElementAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(lh, rh) => write!(f, "{lh}: {rh}"),
            Self::Concat(x) => write!(f, "..{x}"),
        }
    }
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
                write_list!(f, a);
                write!(f, "]")?;
                Ok(())
            }
            Expression::Object(a, _) => {
                write!(f, "{{")?;
                write_list!(f, a);
                write!(f, "}}")
            }
            Expression::Selector { lhs, sel, loc: _ } => write!(f, "{lhs}{sel}"),
            Expression::Constant(c, _) => write!(f, "{c}"),
            Expression::Function { name, args, loc: _ } => {
                write!(f, "{name}(")?;
                write_list!(f, args);
                write!(f, ")")
            }
            Expression::Variable(v, _) => write!(f, "{v}"),
            Expression::Is(i) => write!(f, "({} is {})", i.lhs, i.rhs),
            Expression::If { args, loc: _ } => {
                write!(f, "if ")?;
                write!(f, "{} {{ {} }}", args[0], args[1])?;

                let mut iter = args.iter().skip(2);
                loop {
                    let a1 = iter.next();
                    let a2 = iter.next();

                    match (a1, a2) {
                        (Some(a1), Some(a2)) => write!(f, "else if {} {{ {} }}", a1, a2)?,
                        (Some(a1), None) => write!(f, "else {{ {} }}", a1)?,
                        _ => break,
                    }
                }

                Ok(())
            }
        }
    }
}
