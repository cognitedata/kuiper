mod exec_tree;
mod optimizer;

use std::fmt::Display;

pub use exec_tree::BuildError;
use logos::Span;
pub use optimizer::optimize;

use crate::{
    expressions::ExpressionType, lex::Token, lexer::Lexer, parse::ProgramParser, types::Type,
    CompileError,
};

use self::exec_tree::ExecTreeBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Config for when to run the type checker.
pub enum TypeCheckerMode {
    /// Run the type checker early, before optimization.
    /// This is useful for testing.
    Early,
    /// Run the type checker after optimization.
    Late,
    /// Disable the type checker.
    Off,
}

#[derive(Debug)]
/// Configuration for the compiler.
pub struct CompilerConfig {
    pub(crate) optimizer_operation_limit: i64,
    pub(crate) max_macro_expansions: i32,
    pub(crate) type_checker: TypeCheckerMode,
}

impl CompilerConfig {
    /// Create a new compiler config instance with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of operations during constant execution in the optimizer.
    /// Defaults to 100 000
    pub fn optimizer_operation_limit(mut self, limit: i64) -> Self {
        self.optimizer_operation_limit = limit;
        self
    }

    /// Set the maximum number of macro expansions during compilation. Defaults to 20.
    pub fn max_macro_expansions(mut self, limit: i32) -> Self {
        self.max_macro_expansions = limit;
        self
    }

    /// Set the mode for the type checker. Defaults to Off.
    pub fn type_checker_mode(mut self, mode: TypeCheckerMode) -> Self {
        self.type_checker = mode;
        self
    }
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            optimizer_operation_limit: 100_000,
            max_macro_expansions: 20,
            type_checker: TypeCheckerMode::Off,
        }
    }
}

/// Compile an expression. The `known_inputs` map should contain map from
/// valid input strings to indexes in the input array. You are responsible for ensuring that
/// the expression is run with the correct input array, or it will fail with a source missing error.
///
/// ```
/// use kuiper_lang::compile_expression;
/// use serde_json::json;
///
/// let transform = compile_expression("input.value + 5", &["input"]).unwrap();
///
/// let input = [json!({ "value": 2 })];
/// let result = transform.run(input.iter()).unwrap();
///
/// assert_eq!(result.as_u64().unwrap(), 7);
/// ```
pub fn compile_expression(
    data: &str,
    known_inputs: &[&str],
) -> Result<ExpressionType, CompileError> {
    compile_expression_with_config(data, known_inputs, &Default::default())
}

/// Compile an expression, specifying compiler options. The `known_inputs` map should contain map from
/// valid input strings to indexes in the input array. You are responsible for ensuring that
/// the expression is run with the correct input array, or it will fail with a source missing error.
///
/// ```
/// use kuiper_lang::compile_expression_with_config;
/// use serde_json::json;
///
/// let transform = compile_expression_with_config("input.value + 5", &["input"], &Default::default()).unwrap();
///
/// let input = [json!({ "value": 2 })];
/// let result = transform.run(input.iter()).unwrap();
///
/// assert_eq!(result.as_u64().unwrap(), 7);
/// ```
pub fn compile_expression_with_config(
    data: &str,
    known_inputs: &[&str],
    config: &CompilerConfig,
) -> Result<ExpressionType, CompileError> {
    let inp = Lexer::new(data);
    let parser = ProgramParser::new();
    let res = parser.parse(inp)?;
    let res = ExecTreeBuilder::new(res, known_inputs, config)?.build()?;
    if matches!(config.type_checker, TypeCheckerMode::Early) {
        res.run_types((0..known_inputs.len()).map(|_| Type::Any))?;
    }
    let optimized = optimize(res, known_inputs.len(), config.optimizer_operation_limit)?;
    if matches!(config.type_checker, TypeCheckerMode::Late) {
        optimized.run_types((0..known_inputs.len()).map(|_| Type::Any))?;
    }
    Ok(optimized)
}

/// Compile an expression from an iterator over raw tokens.
///
/// Used for testing and fuzzing.
pub fn compile_from_tokens(
    data: impl Iterator<Item = Token>,
    known_inputs: &[&str],
    config: &CompilerConfig,
) -> Result<ExpressionType, CompileError> {
    let parser = ProgramParser::new();
    let inp = Lexer::new_raw_tokens(data.map(|t| (Ok(t), Span { start: 0, end: 0 })));
    let res = parser.parse(inp)?;
    let res = ExecTreeBuilder::new(res, known_inputs, config)?.build()?;
    if matches!(config.type_checker, TypeCheckerMode::Early) {
        res.run_types((0..known_inputs.len()).map(|_| Type::Any))?;
    }
    let optimized = optimize(res, known_inputs.len(), config.optimizer_operation_limit)?;
    if matches!(config.type_checker, TypeCheckerMode::Late) {
        optimized.run_types((0..known_inputs.len()).map(|_| Type::Any))?;
    }
    Ok(optimized)
}

/// Chunk of debug information about a compilation stage.
#[derive(Debug)]
pub struct DebugInfo {
    #[allow(dead_code)]
    debug: String,
    clean: String,
}

impl Display for DebugInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.clean)
    }
}

/// Debug information about a compilation.
/// When converted to string this shows the state of the compiler at each compilation stage.
#[derive(Debug)]
pub struct ExpressionDebugInfo {
    pub lexer: DebugInfo,
    pub ast: DebugInfo,
    pub exec_tree: DebugInfo,
    pub optimized: DebugInfo,
}

impl Display for ExpressionDebugInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        writeln!(f, "    lexer: {}", self.lexer)?;
        writeln!(f, "    ast: {}", self.ast)?;
        writeln!(f, "    exec_tree: {}", self.exec_tree)?;
        writeln!(f, "    optimized: {}", self.optimized)?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl ExpressionDebugInfo {
    /// Try to compile the input into an expression, and store the compiler state at each stage.
    /// This lets you peek into the compilers interpretation of the program at each stage,
    /// which is useful for debugging.
    ///
    /// `data` is the program itself `known_inputs` is a list of valid input labels.
    pub fn new(
        data: &str,
        known_inputs: &[&str],
        config: &CompilerConfig,
    ) -> Result<Self, CompileError> {
        let lexer = Lexer::new(data);
        let tokens: Result<Vec<_>, _> = lexer.map(|data| data.map(|(_, t, _)| t)).collect();
        let token_info = DebugInfo {
            debug: format!("{tokens:?}"),
            clean: tokens
                .map(|d| d.into_iter().map(|t| t.to_string()).collect())
                .unwrap_or_else(|e| format!("{e:?}")),
        };

        let lexer = Lexer::new(data);
        let parser = ProgramParser::new();
        let ast = parser.parse(lexer)?;
        let ast_info = DebugInfo {
            debug: format!("{ast:?}"),
            clean: ast.to_string(),
        };

        let exec_tree = ExecTreeBuilder::new(ast, known_inputs, config)?.build()?;
        let exec_tree_info = DebugInfo {
            debug: format!("{exec_tree:?}"),
            clean: exec_tree.to_string(),
        };

        let optimized = optimize(
            exec_tree,
            known_inputs.len(),
            config.optimizer_operation_limit,
        )?;
        let optimized_info = DebugInfo {
            debug: format!("{optimized:?}"),
            clean: optimized.to_string(),
        };

        Ok(Self {
            lexer: token_info,
            ast: ast_info,
            exec_tree: exec_tree_info,
            optimized: optimized_info,
        })
    }
}
