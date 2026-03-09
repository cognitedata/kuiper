mod utils;

use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    ops::Range,
    sync::Arc,
};

use gloo_utils::format::JsValueSerdeExt;
use js_sys::{Array, Function, Reflect};
use kuiper_lang::{
    compile_expression_with_config as compile_expression_kuiper, CompileError,
    DynamicFunctionBuilder, Expression, ExpressionMeta, ExpressionType, Span, TransformError,
};
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

impl From<JsValue> for KuiperError {
    fn from(value: JsValue) -> Self {
        Self {
            message: format!("{:?}", value),
            start: None,
            end: None,
        }
    }
}

#[wasm_bindgen]
pub struct KuiperResultWithCompletion {
    result: Value,
    completions: HashMap<Range<usize>, HashSet<String>>,
}

#[wasm_bindgen]
impl KuiperResultWithCompletion {
    pub fn get_completions_at(&self, index: usize) -> Vec<JsValue> {
        let mut res = vec![];
        for (range, completions) in &self.completions {
            if range.start <= index && range.end >= index {
                res.extend(completions.iter().map(|v| JsValue::from_str(v)));
            }
        }
        res
    }

    pub fn get_result(&self) -> Result<JsValue, KuiperError> {
        Ok(JsValue::from_serde(&self.result)?)
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{}, {:?}", &self.result, &self.completions)
    }
}

#[wasm_bindgen]
impl KuiperExpression {
    #[wasm_bindgen(variadic)]
    pub fn run(&self, data: Vec<JsValue>) -> Result<JsValue, KuiperError> {
        let json_items: Vec<Value> = data
            .into_iter()
            .map(|d| d.into_serde())
            .collect::<Result<_, _>>()?;
        let json: Vec<&Value> = json_items.iter().collect();
        let res = self.expression.run(json)?;
        Ok(JsValue::from_serde(&*res)?)
    }

    #[wasm_bindgen(variadic)]
    pub fn run_get_completions(
        &self,
        data: Vec<JsValue>,
    ) -> Result<KuiperResultWithCompletion, KuiperError> {
        let json_items: Vec<Value> = data
            .into_iter()
            .map(|d| d.into_serde())
            .collect::<Result<_, _>>()?;
        let json: Vec<&Value> = json_items.iter().collect();
        let (res, comp) = self.expression.run_get_completions(json)?;
        Ok(KuiperResultWithCompletion {
            result: res.into_owned(),
            completions: comp,
        })
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.expression.to_string()
    }
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn compile_expression(
    data: String,
    inputs: Vec<String>,
    config: Option<CompilerConfig>,
) -> Result<KuiperExpression, KuiperError> {
    let expr = compile_expression_kuiper(
        &data,
        &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
        &config.and_then(|c| c.config).unwrap_or_default(),
    )?;
    Ok(KuiperExpression { expression: expr })
}

#[wasm_bindgen]
pub fn format_expression(input: String) -> Result<String, KuiperError> {
    let formatted = kuiper_lang::format_expression(&input).map_err(|e| KuiperError {
        message: e.to_string(),
        start: None,
        end: None,
    })?;
    Ok(formatted)
}

#[wasm_bindgen]
#[derive(Default)]
pub struct CompilerConfig {
    config: Option<kuiper_lang::CompilerConfig>,
}

fn object_to_string(value: &JsValue) -> Result<String, String> {
    let raw =
        Reflect::get(value, &JsValue::from_str("toString")).map_err(|e| format!("{:?}", e))?;
    let func: &Function = raw.dyn_ref().ok_or_else(|| "Not a function".to_string())?;
    func.call0(value)
        .map(|v| v.as_string().unwrap_or_default())
        .map_err(|e| format!("{:?}", e))
}

#[wasm_bindgen]
impl CompilerConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            config: Some(kuiper_lang::CompilerConfig::default()),
        }
    }

    pub fn set_optimizer_operation_limit(&mut self, limit: i64) {
        self.config = Some(
            self.config
                .take()
                .unwrap_or_default()
                .optimizer_operation_limit(limit),
        );
    }

    pub fn set_max_macro_expansions(&mut self, limit: i32) {
        self.config = Some(
            self.config
                .take()
                .unwrap_or_default()
                .max_macro_expansions(limit),
        );
    }

    pub fn add_custom_function(&mut self, name: String, implementation: js_sys::Function) {
        self.config = Some(
            self.config
                .take()
                .unwrap_or_default()
                .with_custom_dynamic_function(
                    name,
                    Arc::new(CustomBuilder {
                        function: Arc::new(implementation),
                    }),
                ),
        );
    }
}

#[derive(Debug)]
struct Custom {
    function: Arc<js_sys::Function>,
    args: Vec<ExpressionType>,
    span: Span,
}

impl Display for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.function.as_ref().name())
    }
}

impl Expression for Custom {
    fn resolve<'a>(
        &'a self,
        state: &mut kuiper_lang::ExpressionExecutionState<'a, '_>,
    ) -> Result<kuiper_lang::ResolveResult<'a>, TransformError> {
        let args = self
            .args
            .iter()
            .map(|arg| {
                let arg = arg.resolve(state)?;
                JsValue::from_serde(&*arg)
                    .map_err(|e| TransformError::new_invalid_operation(e.to_string(), &self.span))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // This is, frustratingly, the only way to do this.
        let res = match args.as_slice() {
            [] => self.function.call0(&JsValue::NULL),
            [ref a] => self.function.call1(&JsValue::NULL, a),
            [ref a, ref b] => self.function.call2(&JsValue::NULL, a, b),
            [ref a, ref b, ref c] => self.function.call3(&JsValue::NULL, a, b, c),
            [ref a, ref b, ref c, ref d] => self.function.call4(&JsValue::NULL, a, b, c, d),
            [ref a, ref b, ref c, ref d, ref e] => {
                self.function.call5(&JsValue::NULL, a, b, c, d, e)
            }
            [ref a, ref b, ref c, ref d, ref e, ref f] => {
                self.function.call6(&JsValue::NULL, a, b, c, d, e, f)
            }
            [ref a, ref b, ref c, ref d, ref e, ref f, ref g] => {
                self.function.call7(&JsValue::NULL, a, b, c, d, e, f, g)
            }
            [ref a, ref b, ref c, ref d, ref e, ref f, ref g, ref h] => {
                self.function.call8(&JsValue::NULL, a, b, c, d, e, f, g, h)
            }
            _ => self.function.call1(&JsValue::NULL, &Array::from_iter(args)),
        };

        let v: serde_json::Value = res
            .map_err(|e| {
                TransformError::new_invalid_operation(
                    object_to_string(&e).unwrap_or_else(|e| e),
                    &self.span,
                )
            })?
            .into_serde()
            .map_err(|e| TransformError::new_invalid_operation(e.to_string(), &self.span))?;
        Ok(kuiper_lang::ResolveResult::Owned(v))
    }

    fn is_deterministic(&self) -> bool {
        false
    }
}

impl ExpressionMeta for Custom {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.args.iter_mut())
    }
}

struct CustomBuilder {
    function: Arc<js_sys::Function>,
}

impl DynamicFunctionBuilder for CustomBuilder {
    fn make_function(
        &self,
        args: Vec<ExpressionType>,
        span: Span,
    ) -> Result<Box<dyn kuiper_lang::functions::DynamicFunction>, kuiper_lang::BuildError> {
        Ok(Box::new(Custom {
            function: self.function.clone(),
            args,
            span,
        }))
    }
}
