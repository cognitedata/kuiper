use crate::{ParserError, TransformError};

#[derive(Debug)]
pub struct ParserCompileError {
    pub err: ParserError,
    pub id: String,
    pub field: Option<String>,
}

#[derive(Debug)]
pub struct ConfigCompileError {
    pub desc: String,
    pub id: Option<String>,
}

#[derive(Debug)]
pub struct OptimizerCompileError {
    pub err: TransformError,
    pub id: String,
    pub field: Option<String>,
}

#[derive(Debug)]
pub enum CompileError {
    Parser(ParserCompileError),
    Config(ConfigCompileError),
    Optimizer(OptimizerCompileError),
}

impl CompileError {
    pub(crate) fn from_parser_err(err: ParserError, id: &str, field: Option<&str>) -> Self {
        Self::Parser(ParserCompileError {
            err,
            id: id.to_string(),
            field: field.map(|s| s.to_string()),
        })
    }

    pub(crate) fn config_err(desc: &str, id: Option<&str>) -> Self {
        Self::Config(ConfigCompileError {
            desc: desc.to_string(),
            id: id.map(|i| i.to_string()),
        })
    }

    pub(crate) fn optimizer_err(err: TransformError, id: &str, field: Option<&str>) -> Self {
        Self::Optimizer(OptimizerCompileError {
            err,
            id: id.to_string(),
            field: field.map(|s| s.to_string()),
        })
    }
}
