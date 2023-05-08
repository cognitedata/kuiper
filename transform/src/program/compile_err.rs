use crate::{compiler::BuildError, lexer::ParseError, TransformError};

#[derive(Debug)]
pub struct BuildCompileError {
    pub err: BuildError,
    pub id: String,
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
}

#[derive(Debug)]
pub struct ParserCompileError {
    pub err: ParseError,
    pub id: String,
}

#[derive(Debug)]
pub enum CompileError {
    Build(BuildCompileError),
    Parser(ParserCompileError),
    Config(ConfigCompileError),
    Optimizer(OptimizerCompileError),
}

impl CompileError {
    pub(crate) fn from_build_err(err: BuildError, id: &str) -> Self {
        Self::Build(BuildCompileError {
            err,
            id: id.to_string(),
        })
    }

    pub(crate) fn from_parser_err(err: ParseError, id: &str) -> Self {
        Self::Parser(ParserCompileError {
            err,
            id: id.to_string(),
        })
    }

    pub(crate) fn config_err(desc: &str, id: Option<&str>) -> Self {
        Self::Config(ConfigCompileError {
            desc: desc.to_string(),
            id: id.map(|i| i.to_string()),
        })
    }

    pub(crate) fn optimizer_err(err: TransformError, id: &str) -> Self {
        Self::Optimizer(OptimizerCompileError {
            err,
            id: id.to_string(),
        })
    }
}
