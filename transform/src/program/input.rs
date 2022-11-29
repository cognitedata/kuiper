use std::collections::HashMap;

use logos::Logos;
use serde::{Deserialize, Serialize};

use crate::{
    expressions::ExpressionType,
    lexer::Token,
    parse::{Parser, ParserError},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformInput {
    pub id: String,
    pub inputs: Vec<String>,
    pub transform: HashMap<String, String>,
}

enum TransformOrInput {
    Input,
    Transform(usize),
}

pub struct Transform {
    inputs: HashMap<String, TransformOrInput>,
    map: HashMap<String, ExpressionType>,
}

impl Transform {
    fn compile(
        inputs: HashMap<String, TransformOrInput>,
        raw: &TransformInput,
    ) -> Result<Self, CompileError> {
        let mut map = HashMap::new();
        for (key, value) in &raw.transform {
            let inp = Token::lexer(value);
            let result = Parser::new(inp).parse()?;
            map.insert(key.clone(), result);
        }
        Ok(Self { inputs, map })
    }
}

pub struct Program {
    transforms: Vec<Transform>,
}

#[derive(Debug)]
pub enum CompileError {
    Parser(ParserError),
    Config(String),
}

impl From<ParserError> for CompileError {
    fn from(err: ParserError) -> Self {
        Self::Parser(err)
    }
}

impl Program {
    pub fn compile(inp: Vec<TransformInput>) -> Result<Self, CompileError> {
        if inp.is_empty() {
            return Ok(Program { transforms: vec![] });
        }

        let mut transform_map: HashMap<String, usize> = HashMap::new();

        let output = inp.last().unwrap();
        let mut res = vec![];
        Self::compile_rec(&output, &inp, &mut res, &mut transform_map, &vec![])?;

        Ok(Self { transforms: res })

        /* for tf in inp {
            if tf.id == "input" {
                return Err(CompileError::Config("Transform ID may not be \"input\". It is reserved for the input to the pipeline".to_string()));
            }
            if tf.inputs.is_empty() {
                return Err(CompileError::Config((""))
            }
        } */
    }

    fn compile_rec<'a>(
        raw: &'a TransformInput,
        inp: &Vec<TransformInput>,
        build: &mut Vec<Transform>,
        state: &mut HashMap<String, usize>,
        visited: &Vec<&'a String>,
    ) -> Result<(), CompileError> {
        if raw.id == "input" {
            return Err(CompileError::Config(
                "Transform ID may not be \"input\". It is reserved for the input to the pipeline"
                    .to_string(),
            ));
        }
        if raw.inputs.is_empty() {
            return Err(CompileError::Config(format!(
                "Transform with ID {} does not have any inputs",
                raw.id
            )));
        }
        if visited.iter().any(|i| *i == &raw.id) {
            return Err(CompileError::Config(format!(
                "Recursive transformations is not allowed, {} indirectly references itself",
                raw.id
            )));
        }
        if state.contains_key(&raw.id) {
            return Ok(());
        }

        let mut next_visited = visited.clone();
        next_visited.push(&raw.id);
        let mut final_inputs = HashMap::new();
        for input in &raw.inputs {
            if input == "input" {
                final_inputs.insert("input".to_string(), TransformOrInput::Input);
            } else {
                let next = inp.iter().find(|i| &i.id == input).ok_or_else(|| {
                    CompileError::Config(format!("Input {} to {} is not defined", &input, &raw.id))
                })?;

                Self::compile_rec(next, inp, build, state, &next_visited)?;
                final_inputs.insert(
                    input.clone(),
                    TransformOrInput::Transform(*state.get(input).unwrap()),
                );
            }
        }
        if !state.contains_key(&raw.id) {
            build.push(Transform::compile(final_inputs, &raw)?);
            state.insert(raw.id.clone(), build.len() - 1);
        }
        Ok(())
    }
}
