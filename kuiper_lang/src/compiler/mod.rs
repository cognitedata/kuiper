mod exec_tree;
mod optimizer;

use std::{collections::HashMap, fmt::Display};

pub use exec_tree::{from_ast, BuildError};
pub use optimizer::optimize;

use crate::{expressions::ExpressionType, lexer::Lexer, parse::ExprParser, CompileError};

/// Compile an expression. The `known_inputs` map should contain map from
/// valid input strings to indexes in the input array. You are responsible for ensuring that
/// the expression is run with the correct input array, or it will fail with a source missing error.
///
/// ```
/// use kuiper_lang::compile_expression;
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let mut known_inputs = HashMap::new();
/// known_inputs.insert("input".to_string(), 0);
/// let transform = compile_expression("input.value + 5", &mut known_inputs, "my_transform").unwrap();
///
/// let input = [json!({ "value": 2 })];
/// let result = transform.run(input.iter(), "my_transform").unwrap();
///
/// assert_eq!(result.as_u64().unwrap(), 7);
/// ```
pub fn compile_expression(
    data: &str,
    known_inputs: &mut HashMap<String, usize>,
    chunk_id: &str,
) -> Result<ExpressionType, CompileError> {
    let inp = Lexer::new(data);
    let parser = ExprParser::new();
    let res = parser
        .parse(inp)
        .map_err(|e| CompileError::from_parser_err(e, chunk_id))?;
    let res = from_ast(res).map_err(|e| CompileError::from_build_err(e, chunk_id))?;
    let optimized =
        optimize(res, known_inputs).map_err(|e| CompileError::optimizer_err(e, chunk_id))?;
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
    pub fn new(data: &str, known_inputs: &[&str]) -> Result<Self, CompileError> {
        let lexer = Lexer::new(data);
        let tokens: Result<Vec<_>, _> = lexer.map(|data| data.map(|(_, t, _)| t)).collect();
        let token_info = DebugInfo {
            debug: format!("{:?}", tokens),
            clean: tokens
                .map(|d| d.into_iter().map(|t| t.to_string()).collect())
                .unwrap_or_else(|e| format!("{:?}", e)),
        };

        let lexer = Lexer::new(data);
        let parser = ExprParser::new();
        let ast = parser
            .parse(lexer)
            .map_err(|e| CompileError::from_parser_err(e, ""))?;
        let ast_info = DebugInfo {
            debug: format!("{:?}", ast),
            clean: ast.to_string(),
        };

        let exec_tree = from_ast(ast).map_err(|e| CompileError::from_build_err(e, ""))?;
        let exec_tree_info = DebugInfo {
            debug: format!("{:?}", exec_tree),
            clean: exec_tree.to_string(),
        };

        let mut inputs = HashMap::new();
        for (idx, &inp) in known_inputs.iter().enumerate() {
            inputs.insert(inp.to_string(), idx);
        }
        let optimized =
            optimize(exec_tree, &mut inputs).map_err(|e| CompileError::optimizer_err(e, ""))?;
        let optimized_info = DebugInfo {
            debug: format!("{:?}", optimized),
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
