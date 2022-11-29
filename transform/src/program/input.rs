use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use logos::Logos;
use serde::{Deserialize, Serialize};

use crate::{
    expressions::ExpressionType,
    lexer::Token,
    parse::{Parser, ParserError},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapTransformInput {
    pub id: String,
    pub inputs: Vec<String>,
    pub transform: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum TransformInput {
    Map(MapTransformInput),
    Flatten(FlattenTransformInput),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlattenTransformInput {
    pub id: String,
    pub inputs: Vec<String>,
    pub transform: String,
}

impl TransformInput {
    pub fn id(&self) -> &String {
        match self {
            Self::Map(x) => &x.id,
            Self::Flatten(x) => &x.id,
        }
    }

    pub fn inputs(&self) -> &Vec<String> {
        match self {
            Self::Map(x) => &x.inputs,
            Self::Flatten(x) => &x.inputs,
        }
    }
}

pub(crate) struct TransformInputs {
    pub inputs: HashMap<String, TransformOrInput>,
    pub used_inputs: HashSet<TransformOrInput>,
}

impl TransformInputs {
    pub fn new(inputs: HashMap<String, TransformOrInput>) -> Self {
        let set = HashSet::from_iter(inputs.iter().map(|(_, v)| v.clone()));
        Self {
            inputs,
            used_inputs: set,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub enum TransformOrInput {
    Input,
    Transform(usize),
}

impl Display for TransformOrInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => write!(f, "input"),
            Self::Transform(u) => write!(f, "{}", u),
        }
    }
}

pub struct MapTransform {
    inputs: TransformInputs,
    pub(crate) map: HashMap<String, ExpressionType>,
}

pub struct FlattenTransform {
    inputs: TransformInputs,
    pub(crate) map: ExpressionType,
}

pub enum Transform {
    Map(MapTransform),
    Flatten(FlattenTransform),
}

impl Transform {
    pub(crate) fn inputs(&self) -> &TransformInputs {
        match self {
            Transform::Map(x) => &x.inputs,
            Transform::Flatten(x) => &x.inputs,
        }
    }

    fn compile(
        inputs: HashMap<String, TransformOrInput>,
        raw: &TransformInput,
    ) -> Result<Self, CompileError> {
        let mut map = HashMap::new();
        match raw {
            TransformInput::Map(raw) => {
                for (key, value) in &raw.transform {
                    let inp = Token::lexer(value);
                    let result = Parser::new(inp).parse()?;
                    map.insert(key.clone(), result);
                }
                Ok(Self::Map(MapTransform {
                    inputs: TransformInputs::new(inputs),
                    map,
                }))
            }
            TransformInput::Flatten(raw) => {
                let inp = Token::lexer(&raw.transform);
                let result = Parser::new(inp).parse()?;
                Ok(Self::Flatten(FlattenTransform {
                    inputs: TransformInputs::new(inputs),
                    map: result,
                }))
            }
        }
    }
}

pub struct Program {
    pub(crate) transforms: Vec<Transform>,
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
        if raw.id() == "input" {
            return Err(CompileError::Config(
                "Transform ID may not be \"input\". It is reserved for the input to the pipeline"
                    .to_string(),
            ));
        }
        if visited.iter().any(|i| *i == raw.id()) {
            return Err(CompileError::Config(format!(
                "Recursive transformations is not allowed, {} indirectly references itself",
                raw.id()
            )));
        }
        if state.contains_key(raw.id()) {
            return Ok(());
        }

        let mut next_visited = visited.clone();
        next_visited.push(raw.id());
        let mut final_inputs = HashMap::new();
        for input in raw.inputs() {
            if input == "input" {
                final_inputs.insert("input".to_string(), TransformOrInput::Input);
            } else {
                let next = inp.iter().find(|i| i.id() == input).ok_or_else(|| {
                    CompileError::Config(format!("Input {} to {} is not defined", &input, raw.id()))
                })?;

                Self::compile_rec(next, inp, build, state, &next_visited)?;
                final_inputs.insert(
                    input.clone(),
                    TransformOrInput::Transform(*state.get(input).unwrap()),
                );
            }
        }
        if !state.contains_key(raw.id()) {
            build.push(Transform::compile(final_inputs, &raw)?);
            state.insert(raw.id().clone(), build.len() - 1);
        }
        Ok(())
    }
}
