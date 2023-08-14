mod utils;

use gloo_utils::format::JsValueSerdeExt;
use kuiper_lang::{compile_expression as compile_expression_kuiper, CompileError, TransformError};
use serde_json::Value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
pub struct KuiperError {
    pub message: String,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

#[wasm_bindgen]
pub struct KuiperExpression {
    expression: kuiper_lang::ExpressionType,
}

impl From<TransformError> for KuiperError {
    fn from(value: TransformError) -> Self {
        Self {
            message: value.to_string(),
            start: value.span().map(|s| s.start),
            end: value.span().map(|s| s.end),
        }
    }
}

impl From<CompileError> for KuiperError {
    fn from(value: CompileError) -> Self {
        Self {
            message: value.to_string(),
            start: value.span().map(|s| s.start),
            end: value.span().map(|s| s.end),
        }
    }
}

impl From<serde_json::Error> for KuiperError {
    fn from(value: serde_json::Error) -> Self {
        Self {
            message: value.to_string(),
            start: Some(value.column()),
            end: Some(value.column()),
        }
    }
}

#[wasm_bindgen]
impl KuiperExpression {
    pub fn run(&self, data: JsValue) -> Result<JsValue, KuiperError> {
        let json_item: Value = data.into_serde()?;
        let json = vec![&json_item];
        let res = self.expression.run(json)?;
        Ok(JsValue::from_serde(&res)?)
    }

    pub fn run_multiple_inputs(&self, data: Vec<JsValue>) -> Result<JsValue, KuiperError> {
        let json_items: Vec<Value> = data
            .into_iter()
            .map(|d| d.into_serde())
            .collect::<Result<_, _>>()?;
        let json: Vec<&Value> = json_items.iter().collect();
        let res = self.expression.run(json)?;
        Ok(JsValue::from_serde(&res)?)
    }
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn compile_expression(
    data: String,
    inputs: Vec<JsValue>,
) -> Result<KuiperExpression, KuiperError> {
    let input_strings = inputs
        .into_iter()
        .map(|j| {
            j.as_string().ok_or_else(|| KuiperError {
                message: "Inputs must be string".to_string(),
                start: None,
                end: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expr = compile_expression_kuiper(
        &data,
        &input_strings.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    Ok(KuiperExpression { expression: expr })
}
