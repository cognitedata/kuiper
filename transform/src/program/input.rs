use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use logos::Logos;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    expressions::{optimize, ExpressionType},
    lexer::Token,
    parse::Parser,
};

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
    Flatten(FlatTransformInput),
    /// Filters transform input, returns input values if the output is "true", skips if "false" or "null".
    Filter(FlatTransformInput),
}

/// Input to a "flat" transform.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatTransformInput {
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
            Self::Filter(x) => &x.id,
        }
    }

    pub fn inputs(&self) -> &Vec<String> {
        match self {
            Self::Map(x) => &x.inputs,
            Self::Flatten(x) => &x.inputs,
            Self::Filter(x) => &x.inputs,
        }
    }

    pub fn mode(&self) -> TransformInputType {
        match self {
            Self::Map(x) => x.mode,
            Self::Flatten(x) => x.mode,
            Self::Filter(x) => x.mode,
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
        let set = HashSet::from_iter(inputs.values().cloned());
        Self {
            inputs,
            used_inputs: set,
            mode,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub enum TransformOrInput {
    Input(usize),
    Transform(usize),
    Merge,
}

impl Display for TransformOrInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(u) => write!(f, "input{u}"),
            Self::Transform(u) => write!(f, "{u}"),
            Self::Merge => write!(f, "merge"),
        }
    }
}

pub struct MapTransform {
    inputs: TransformInputs,
    id: String,
    pub(crate) map: HashMap<String, ExpressionType>,
}

pub struct FlatTransform {
    inputs: TransformInputs,
    id: String,
    pub(crate) map: ExpressionType,
}

pub enum Transform {
    Map(MapTransform),
    Flatten(FlatTransform),
    Filter(FlatTransform),
}

impl Transform {
    pub(crate) fn inputs(&self) -> &TransformInputs {
        match self {
            Transform::Map(x) => &x.inputs,
            Transform::Flatten(x) => &x.inputs,
            Transform::Filter(x) => &x.inputs,
        }
    }

    pub(crate) fn id(&self) -> &str {
        match self {
            Transform::Map(x) => &x.id,
            Transform::Flatten(x) => &x.id,
            Transform::Filter(x) => &x.id,
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
                    let result = optimize(result)
                        .map_err(|e| CompileError::optimizer_err(e, &raw.id, Some(key)))?;
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
                let result =
                    optimize(result).map_err(|e| CompileError::optimizer_err(e, &raw.id, None))?;
                Ok(Self::Flatten(FlatTransform {
                    inputs: TransformInputs::new(inputs, raw.mode),
                    map: result,
                    id: raw.id.clone(),
                }))
            }
            TransformInput::Filter(raw) => {
                let inp = Token::lexer(&raw.transform);
                let result = Parser::new(inp)
                    .parse()
                    .map_err(|e| CompileError::from_parser_err(e, &raw.id, None))?;
                let result =
                    optimize(result).map_err(|e| CompileError::optimizer_err(e, &raw.id, None))?;
                if inputs.len() != 1
                    && (inputs.is_empty() || !matches!(raw.mode, TransformInputType::Merge))
                {
                    return Err(CompileError::config_err(
                        "Filter operations must have exactly one input or use input mode \"merge\"",
                        Some(&raw.id),
                    ));
                }
                Ok(Self::Filter(FlatTransform {
                    inputs: TransformInputs::new(inputs, raw.mode),
                    id: raw.id.clone(),
                    map: result,
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
    /// input_aliases are aliases in code for inputs in position given by the key. By default they have keys input0, input1, etc.
    /// and the special "input" alias for input0. This list can add new aliases at each index. Make sure not to add an alias input or merge,
    /// as that may have unpredictable effects.
    pub fn compile_map(
        inp: Vec<TransformInput>,
        input_aliases: &HashMap<usize, Vec<String>>,
    ) -> Result<Self, CompileError> {
        if inp.is_empty() {
            return Ok(Program { transforms: vec![] });
        }

        let mut transform_map: HashMap<String, usize> = HashMap::new();
        let mut inv_input_map: HashMap<String, usize> = HashMap::new();
        for (key, value) in input_aliases {
            for alias in value {
                inv_input_map.insert(alias.clone(), *key);
            }
        }

        let output = inp.last().unwrap();
        let mut res = vec![];
        Self::compile_rec(
            output,
            &inp,
            &mut res,
            &mut transform_map,
            &[],
            &input_aliases,
            &inv_input_map,
        )?;

        Ok(Self { transforms: res })
    }

    /// Compile the program. The input is a list of raw transform inputs, which should have unique IDs.
    pub fn compile(inp: Vec<TransformInput>) -> Result<Self, CompileError> {
        if inp.is_empty() {
            return Ok(Program { transforms: vec![] });
        }

        let mut transform_map: HashMap<String, usize> = HashMap::new();

        let output = inp.last().unwrap();
        let mut res = vec![];
        Self::compile_rec(
            output,
            &inp,
            &mut res,
            &mut transform_map,
            &[],
            &HashMap::new(),
            &HashMap::new(),
        )?;

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
        input_map: &HashMap<usize, Vec<String>>,
        inv_input_map: &HashMap<String, usize>,
    ) -> Result<(), CompileError> {
        lazy_static::lazy_static! {
            static ref INPUT_RE: Regex = Regex::new("^input([0-9]*)$").unwrap();
        }

        if inv_input_map.contains_key(raw.id()) {
            let values_str = input_map
                .values()
                .flat_map(|v| v)
                .fold("\"merge\"".to_owned(), |a, b| {
                    a + ", " + &format!("\"{b}\"")
                });
            return Err(CompileError::config_err(&format!("Transform ID may not start with \"input\" or be equal to any special inputs: {}", values_str), Some(raw.id())));
        }

        if raw.id().starts_with("input") || raw.id() == "merge" {
            return Err(CompileError::config_err(
                "Transform ID may not start with \"input\" or be equal to \"merge\". They are reserved for special inputs to the pipeline",
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
            if input.starts_with("input") {
                let caps = INPUT_RE.captures(input);
                let Some(caps) = caps else {
                    return Err(CompileError::config_err("Transform inputs starting with \"input\" must be either just \"input\" or \"input123\"", Some(raw.id())));
                };
                let idx: usize = if caps.len() == 2 {
                    let cap = caps.get(1).unwrap().as_str();
                    if cap.is_empty() {
                        0
                    } else {
                        cap.parse().map_err(|_| {
                            CompileError::config_err(
                                &format!("Invalid transform input: {input}"),
                                Some(raw.id()),
                            )
                        })?
                    }
                } else {
                    0
                };

                final_inputs.insert(format!("input{idx}"), TransformOrInput::Input(idx));
                if idx == 0 {
                    final_inputs.insert("input".to_string(), TransformOrInput::Input(0));
                }
                if let Some(aliases) = input_map.get(&idx) {
                    for alias in aliases.iter() {
                        final_inputs.insert(alias.clone(), TransformOrInput::Input(idx));
                    }
                }
            } else if let Some(idx) = inv_input_map.get(input) {
                final_inputs.insert(input.clone(), TransformOrInput::Input(*idx));
                final_inputs.insert(format!("input{idx}"), TransformOrInput::Input(*idx));
                if *idx == 0 {
                    final_inputs.insert("input".to_string(), TransformOrInput::Input(0));
                }
            } else {
                let next = inp.iter().find(|i| i.id() == input).ok_or_else(|| {
                    CompileError::config_err(
                        &format!("Input {} to {} is not defined", &input, raw.id()),
                        Some(raw.id()),
                    )
                })?;

                Self::compile_rec(
                    next,
                    inp,
                    build,
                    state,
                    &next_visited,
                    &input_map,
                    &inv_input_map,
                )?;
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
