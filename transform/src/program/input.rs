use std::{collections::HashMap, fmt::Display};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{compiler::compile_expression, expressions::ExpressionType};

use super::compile_err::CompileError;

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

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum TransformType {
    #[default]
    Map,
    Filter,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformInput {
    /// A unique id for this transform. Verify that the ID is unique before passing to the compiler.
    pub id: String,
    /// A list of inputs. May be empty. "input" is a magic input, it indicates the source of the transformation.
    pub inputs: Vec<String>,
    /// The transformation, a single expression. If the output is a JSON array it will be converted into an array of outputs, if flatten is set to true.
    pub transform: String,
    /// How the inputs are combined
    #[serde(default)]
    pub mode: TransformInputType,
    /// True to flatten the output into an output for each array element.
    #[serde(default)]
    pub expand_output: bool,
    /// Transform type.
    #[serde(default)]
    pub r#type: TransformType,
}

/// Container for information about the input to the transform.
pub(crate) struct TransformInputs {
    /// The inputs to this transform, maps from a name to an index in the transform array, or to the input to the program.
    pub inputs: HashMap<String, usize>,
    /// For convenience, a set of the inputs used in this transform.
    pub used_inputs: Vec<TransformOrInput>,
    /// Type of transform input.
    pub mode: TransformInputType,
}

impl TransformInputs {
    pub fn new(
        raw_inputs: &[String],
        inputs: HashMap<String, TransformOrInput>,
        mode: TransformInputType,
    ) -> Self {
        let used_inputs: Vec<_> = raw_inputs
            .iter()
            .map(|l| inputs.get(l).unwrap().clone())
            .collect();
        if matches!(mode, TransformInputType::Merge) {
            let mut input_indexes = HashMap::new();
            for (inp, id) in inputs {
                if id == TransformOrInput::Merge {
                    input_indexes.insert(inp, 0);
                }
            }
            return Self {
                inputs: input_indexes,
                used_inputs,
                mode,
            };
        }

        let mut input_indexes = HashMap::with_capacity(inputs.len());
        for (inp, id) in inputs {
            let idx = used_inputs.iter().position(|i| i == &id);
            if let Some(idx) = idx {
                input_indexes.insert(inp, idx);
            }
        }
        Self {
            inputs: input_indexes,
            used_inputs,
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

impl TransformOrInput {
    pub fn get_index(&self, num_inputs: usize) -> usize {
        match self {
            TransformOrInput::Input(i) => *i,
            TransformOrInput::Transform(i) => *i + num_inputs,
            TransformOrInput::Merge => 0,
        }
    }
}

pub struct Transform {
    pub(crate) inputs: TransformInputs,
    pub(crate) id: String,
    pub(crate) map: ExpressionType,
    pub(crate) flatten: bool,
    pub(crate) transform_type: TransformType,
}

impl Transform {
    /// Compile a transform. The input is a map of the inputs, and the raw transform input.
    /// This just builds lexer and parser for each step.
    fn compile(
        inputs: HashMap<String, TransformOrInput>,
        raw: &TransformInput,
    ) -> Result<Self, CompileError> {
        if matches!(raw.r#type, TransformType::Filter)
            && inputs.len() != 1
            && (inputs.is_empty() || !matches!(raw.mode, TransformInputType::Merge))
        {
            return Err(CompileError::config_err(
                "Filter operations must have exactly one input or use input mode \"merge\"",
                Some(&raw.id),
            ));
        }
        let mut inputs = TransformInputs::new(&raw.inputs, inputs, raw.mode);

        let result = compile_expression(&raw.transform, &mut inputs.inputs, &raw.id)?;

        Ok(Self {
            inputs,
            id: raw.id.clone(),
            map: result,
            flatten: raw.expand_output,
            transform_type: raw.r#type,
        })
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
            input_aliases,
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

        if inv_input_map.contains_key(&raw.id) {
            let values_str = input_map
                .values()
                .flatten()
                .fold("\"merge\"".to_owned(), |a, b| {
                    a + ", " + &format!("\"{b}\"")
                });
            return Err(CompileError::config_err(&format!("Transform ID may not start with \"input\" or be equal to any special inputs: {}", values_str), Some(&raw.id)));
        }

        if raw.id.starts_with("input") || raw.id == "merge" {
            return Err(CompileError::config_err(
                "Transform ID may not start with \"input\" or be equal to \"merge\". They are reserved for special inputs to the pipeline",
                Some(&raw.id)
            ));
        }
        if visited.iter().any(|i| *i == &raw.id) {
            return Err(CompileError::config_err(
                &format!(
                    "Recursive transformations are not allowed, {} indirectly references itself",
                    raw.id
                ),
                Some(&raw.id),
            ));
        }
        if state.contains_key(&raw.id) {
            return Ok(());
        }

        let mut next_visited = visited.to_owned();
        next_visited.push(&raw.id);
        let mut final_inputs = HashMap::new();
        for input in &raw.inputs {
            if input.starts_with("input") {
                let caps = INPUT_RE.captures(input);
                let Some(caps) = caps else {
                    return Err(CompileError::config_err("Transform inputs starting with \"input\" must be either just \"input\" or \"input123\"", Some(&raw.id)));
                };
                let idx: usize = if caps.len() == 2 {
                    let cap = caps.get(1).unwrap().as_str();
                    if cap.is_empty() {
                        0
                    } else {
                        cap.parse().map_err(|_| {
                            CompileError::config_err(
                                &format!("Invalid transform input: {input}"),
                                Some(&raw.id),
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
                let next = inp.iter().find(|i| &i.id == input).ok_or_else(|| {
                    CompileError::config_err(
                        &format!("Input {} to {} is not defined", &input, raw.id),
                        Some(&raw.id),
                    )
                })?;

                Self::compile_rec(
                    next,
                    inp,
                    build,
                    state,
                    &next_visited,
                    input_map,
                    inv_input_map,
                )?;
                final_inputs.insert(
                    input.clone(),
                    TransformOrInput::Transform(*state.get(input).unwrap()),
                );
            }
        }
        if matches!(raw.mode, TransformInputType::Merge) {
            final_inputs.insert("merge".to_string(), TransformOrInput::Merge);
        }
        if !state.contains_key(&raw.id) {
            build.push(Transform::compile(final_inputs, raw)?);
            state.insert(raw.id.clone(), build.len() - 1);
        }
        Ok(())
    }
}
