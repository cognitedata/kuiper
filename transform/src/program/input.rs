use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use logos::Logos;
use serde::{Deserialize, Serialize};

use crate::{expressions::ExpressionType, lexer::Token, parse::Parser};

use super::compile_err::CompileError;

/// Input to a "map" transform.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapTransformInput {
    /// A unique id for this transform. Verify that the ID is unique before passing to the compiler.
    pub id: String,
    /// A list of inputs. May be empty. "input" is a magic input, it indicates the source of the transformation.
    pub inputs: Vec<String>,
    /// The transformation, a map from output field name to expression.
    pub transform: HashMap<String, String>,
    /// How the inputs are combined
    #[serde(default)]
    pub mode: TransformInputType,
}

/// How multiple inputs are handled in transformations.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum TransformInputType {
    /// Default, a cross product is produced of the inputs. So [1, 2], [3, 4] becomes
    /// [1, 3], [1, 4], [2, 3], [2, 4]
    Product,
    /// Merge, all inputs are chained after each other, so [1, 2] and [3, 4] becomes
    /// [1, 2, 3, 4]. The new input name is `merge`, so $merge will resolve to 1, 2, 3, 4 for each run of the
    /// transformation respectively.
    Merge,
    /// Zip, inputs are grouped by index, so [1, 2] and [3, 4] becomes [1, 3] and [2, 4].
    /// Requires all inputs to be the same length.
    Zip,
}

impl Default for TransformInputType {
    fn default() -> Self {
        Self::Product
    }
}

/// Input to a transform. A program consists of a list of these.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum TransformInput {
    /// Map transform, outputs a JSON object.
    Map(MapTransformInput),
    /// Flatten transform, outputs an array of JSON values.
    Flatten(FlattenTransformInput),
}

/// Input to a "flatten" transform.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlattenTransformInput {
    /// A unique id for this transform. Verify that the ID is unique before passing to the compiler.
    pub id: String,
    /// A list of inputs. May be empty. "input" is a magic input, it indicates the source of the transformation.
    pub inputs: Vec<String>,
    /// The transformation, a single expression. If the output is a JSON array it will be converted into an array of outputs.
    pub transform: String,
    /// How the inputs are combined
    #[serde(default)]
    pub mode: TransformInputType,
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

    pub fn mode(&self) -> TransformInputType {
        match self {
            Self::Map(x) => x.mode,
            Self::Flatten(x) => x.mode,
        }
    }
}

/// Container for information about the input to the transform.
pub(crate) struct TransformInputs {
    /// The inputs to this transform, maps from a name to an index in the transform array, or to the input to the program.
    pub inputs: HashMap<String, TransformOrInput>,
    /// For convenience, a set of the inputs used in this transform.
    pub used_inputs: HashSet<TransformOrInput>,
    /// Type of transform input.
    pub mode: TransformInputType,
}

impl TransformInputs {
    pub fn new(inputs: HashMap<String, TransformOrInput>, mode: TransformInputType) -> Self {
        let set = HashSet::from_iter(inputs.iter().map(|(_, v)| v.clone()));
        Self {
            inputs,
            used_inputs: set,
            mode,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub enum TransformOrInput {
    Input,
    Transform(usize),
    Merge,
}

impl Display for TransformOrInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => write!(f, "input"),
            Self::Transform(u) => write!(f, "{}", u),
            Self::Merge => write!(f, "merge"),
        }
    }
}

pub struct MapTransform {
    inputs: TransformInputs,
    id: String,
    pub(crate) map: HashMap<String, ExpressionType>,
}

pub struct FlattenTransform {
    inputs: TransformInputs,
    id: String,
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

    pub(crate) fn id(&self) -> &str {
        match self {
            Transform::Map(x) => &x.id,
            Transform::Flatten(x) => &x.id,
        }
    }

    /// Compile a transform. The input is a map of the inputs, and the raw transform input.
    /// This just builds lexer and parser for each step.
    fn compile(
        inputs: HashMap<String, TransformOrInput>,
        raw: &TransformInput,
    ) -> Result<Self, CompileError> {
        let mut map = HashMap::new();
        match raw {
            TransformInput::Map(raw) => {
                for (key, value) in &raw.transform {
                    let inp = Token::lexer(value);
                    let result = Parser::new(inp)
                        .parse()
                        .map_err(|e| CompileError::from_parser_err(e, &raw.id, Some(key)))?;
                    map.insert(key.clone(), result);
                }
                Ok(Self::Map(MapTransform {
                    inputs: TransformInputs::new(inputs, raw.mode),
                    map,
                    id: raw.id.clone(),
                }))
            }
            TransformInput::Flatten(raw) => {
                let inp = Token::lexer(&raw.transform);
                let result = Parser::new(inp)
                    .parse()
                    .map_err(|e| CompileError::from_parser_err(e, &raw.id, None))?;
                Ok(Self::Flatten(FlattenTransform {
                    inputs: TransformInputs::new(inputs, raw.mode),
                    map: result,
                    id: raw.id.clone(),
                }))
            }
        }
    }
}

/// The actual compiled program itself.
pub struct Program {
    pub(crate) transforms: Vec<Transform>,
}

impl Program {
    /// Compile the program. The input is a list of raw transform inputs, which should have unique IDs.
    pub fn compile(inp: Vec<TransformInput>) -> Result<Self, CompileError> {
        if inp.is_empty() {
            return Ok(Program { transforms: vec![] });
        }

        let mut transform_map: HashMap<String, usize> = HashMap::new();

        let output = inp.last().unwrap();
        let mut res = vec![];
        Self::compile_rec(output, &inp, &mut res, &mut transform_map, &[])?;

        Ok(Self { transforms: res })
    }

    /// Recursive compilation. Instead of just compiling each transformation we recurse from the last transform,
    /// which is the output. That way we avoid compiling transformations that will never be used.
    ///
    /// `raw` is the current transform
    /// `inp` is the full list of transform inputs.
    /// `build` is the built list of transforms
    /// `state` is a map from transform ID to index in the build array.
    /// `visited` is a dynamic list containing the IDs visited in this branch of the recursion tree.
    ///
    /// We do not want to allow recursion, since that can have some unpleasant effects and make it too easy
    /// to create non-terminating programs.
    fn compile_rec<'a>(
        raw: &'a TransformInput,
        inp: &Vec<TransformInput>,
        build: &mut Vec<Transform>,
        state: &mut HashMap<String, usize>,
        visited: &[&'a String],
    ) -> Result<(), CompileError> {
        if raw.id() == "input" || raw.id() == "merge" {
            return Err(CompileError::config_err(
                "Transform ID may not be \"input\" or \"merge\". They are reserved for special inputs to the pipeline",
                Some(raw.id())
            ));
        }
        if visited.iter().any(|i| *i == raw.id()) {
            return Err(CompileError::config_err(
                &format!(
                    "Recursive transformations are not allowed, {} indirectly references itself",
                    raw.id()
                ),
                Some(raw.id()),
            ));
        }
        if state.contains_key(raw.id()) {
            return Ok(());
        }

        let mut next_visited = visited.to_owned();
        next_visited.push(raw.id());
        let mut final_inputs = HashMap::new();
        for input in raw.inputs() {
            if input == "input" {
                final_inputs.insert("input".to_string(), TransformOrInput::Input);
            } else {
                let next = inp.iter().find(|i| i.id() == input).ok_or_else(|| {
                    CompileError::config_err(
                        &format!("Input {} to {} is not defined", &input, raw.id()),
                        Some(raw.id()),
                    )
                })?;

                Self::compile_rec(next, inp, build, state, &next_visited)?;
                final_inputs.insert(
                    input.clone(),
                    TransformOrInput::Transform(*state.get(input).unwrap()),
                );
            }
        }
        if matches!(raw.mode(), TransformInputType::Merge) {
            final_inputs.insert("merge".to_string(), TransformOrInput::Merge);
        }
        if !state.contains_key(raw.id()) {
            build.push(Transform::compile(final_inputs, raw)?);
            state.insert(raw.id().clone(), build.len() - 1);
        }
        Ok(())
    }
}
