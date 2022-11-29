use std::collections::HashMap;

use serde_json::Value;

use crate::expressions::{ResolveResult, TransformError};

use super::{input::TransformOrInput, Program};

impl Program {
    pub fn execute(&self, input: Value) -> Result<Value, TransformError> {
        let mut result = HashMap::<TransformOrInput, ResolveResult>::new();
        result.insert(TransformOrInput::Input, ResolveResult::Reference(&input));

        let len = self.transforms.len();
        for (idx, tf) in self.transforms.iter().enumerate() {
            let value = tf.execute(&result)?;
            if idx == len - 1 {
                return Ok(value);
            }
            // cached_results.insert(idx, value);
            result.insert(
                TransformOrInput::Transform(idx),
                ResolveResult::Value(value),
            );
        }
        Err(TransformError::SourceMissingError(
            "No transforms in program".to_string(),
        ))
    }
}
