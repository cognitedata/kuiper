use thiserror::Error;

use crate::{compiler::BuildError, lexer::ParseError, TransformError};

#[derive(Debug, Error)]
pub struct SubCompileError<T: std::error::Error + std::fmt::Debug> {
    pub err: T,
    pub id: String,
}

impl<T> std::fmt::Display for SubCompileError<T>
where
    T: std::error::Error + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}. ID: {}", self.err, self.id)
    }
}

#[derive(Debug, Error)]
pub struct ConfigCompileError {
    pub desc: String,
    pub id: Option<String>,
}

impl std::fmt::Display for ConfigCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = &self.id {
            write!(f, "{}. ID: {id}", self.desc)
        } else {
            write!(f, "{}", self.desc)
        }
    }
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Compilation failed: {0}")]
    Build(SubCompileError<BuildError>),
    #[error("Compilation failed: {0}")]
    Parser(SubCompileError<ParseError>),
    #[error("Compilation failed: {0}")]
    Config(ConfigCompileError),
    #[error("Compilation failed: {0}")]
    Optimizer(SubCompileError<TransformError>),
}

impl CompileError {
    pub(crate) fn from_build_err(err: BuildError, id: &str) -> Self {
        Self::Build(SubCompileError {
            err,
            id: id.to_string(),
        })
    }

    pub(crate) fn from_parser_err(err: ParseError, id: &str) -> Self {
        Self::Parser(SubCompileError {
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
        Self::Optimizer(SubCompileError {
            err,
            id: id.to_string(),
        })
    }
}
