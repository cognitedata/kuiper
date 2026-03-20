use std::{
    ffi::{c_char, c_void, CStr, CString},
    fmt::Display,
    ptr::slice_from_raw_parts,
    sync::Arc,
};

use kuiper_lang::{
    CompileError, DynamicFunctionBuilder, Expression, ExpressionMeta, ExpressionType, Span,
    TransformError,
};
use serde_json::Value;
use thiserror::Error;

#[repr(C)]
#[derive(Debug)]
pub struct CompileResult {
    pub error: KuiperError,
    pub result: *mut ExpressionType,
}

#[repr(C)]
#[derive(Debug)]
pub struct KuiperError {
    pub error: *mut c_char,
    pub is_error: bool,
    pub start: u64,
    pub end: u64,
}

#[derive(Error, Debug)]
enum InteropError {
    #[error("{0}")]
    Compile(#[from] CompileError),
    #[error("{0}")]
    Execute(#[from] TransformError),
    #[error("Input must be valid JSON: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Invalid string input: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

unsafe fn compile_expression_internal(
    data: *const c_char,
    inputs: *const *const c_char,
    len: usize,
    config: &kuiper_lang::CompilerConfig,
) -> Result<ExpressionType, InteropError> {
    let data = unsafe { CStr::from_ptr(data) };
    let inputs = if len > 0 {
        let inputs_raw = unsafe { &*slice_from_raw_parts(inputs, len) };
        inputs_raw
            .iter()
            .map(|i| CStr::from_ptr(*i).to_str())
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    Ok(kuiper_lang::compile_expression_with_config(
        data.to_str()?,
        &inputs,
        config,
    )?)
}

/// Destroy a compile result. Called from external code to correctly free rust allocated memory.
///
/// # Safety
///
/// `data` must be a valid, non-null rust pointer to a `CompileResult`, typically produced by `compile_expression`
#[no_mangle]
pub unsafe extern "C" fn destroy_compile_result(data: *mut CompileResult) {
    let data = unsafe { Box::from_raw(data) };
    if !data.error.error.is_null() {
        unsafe { std::mem::drop(CString::from_raw(data.error.error)) };
    }
    if !data.result.is_null() {
        unsafe { drop(Box::from_raw(data.result)) };
    }
}

/// Destroy an expression type
///
/// # Safety
///
/// `data` must be a valid, non-null rust pointer to an `ExpressionType`, typically produced by
/// `compile_expression` and `get_expression_from_compile_result`.
#[no_mangle]
pub unsafe extern "C" fn destroy_expression(data: *mut ExpressionType) {
    unsafe { drop(Box::from_raw(data)) };
}

/// Destroy a `CompileResult` and return the `ExpressionType` it contains.
/// This does not check whether `result` is null and may return a null pointer.
///
/// # Safety
///
/// `data` must be a valid rust pointer to a `CompileResult`, typically produced by
/// `compile_expression`.
#[no_mangle]
pub unsafe extern "C" fn get_expression_from_compile_result(
    data: *mut CompileResult,
) -> *mut ExpressionType {
    let data = unsafe { Box::from_raw(data) };
    if !data.error.error.is_null() {
        unsafe { drop(CString::from_raw(data.error.error)) };
    }
    data.result
}

impl From<InteropError> for KuiperError {
    fn from(value: InteropError) -> Self {
        match value {
            InteropError::Compile(c) => KuiperError {
                is_error: true,
                error: CString::new(c.to_string()).unwrap().into_raw(),
                start: c.span().map(|s| s.start as u64).unwrap_or_default(),
                end: c.span().map(|s| s.end as u64).unwrap_or_default(),
            },
            InteropError::Execute(c) => KuiperError {
                is_error: true,
                error: CString::new(c.to_string()).unwrap().into_raw(),
                start: c.span().map(|s| s.start as u64).unwrap_or_default(),
                end: c.span().map(|s| s.end as u64).unwrap_or_default(),
            },
            c => KuiperError {
                is_error: true,
                error: CString::new(c.to_string()).unwrap().into_raw(),
                start: 0,
                end: 0,
            },
        }
    }
}

/// Compile a kuiper expression from a string and a list of inputs.
///
/// Returns a result struct in which exactly one of `error` or `result` is non-null.
///
/// # Safety
///
/// `data` must be a valid, utf8-encoded, null terminated string. `inputs` must be an array of such strings
/// with length `len`. If `len` is 0, `inputs` may be null.
#[no_mangle]
pub unsafe extern "C" fn compile_expression(
    data: *const c_char,
    inputs: *const *const c_char,
    len: usize,
) -> *mut CompileResult {
    let res = match compile_expression_internal(
        data,
        inputs,
        len,
        &kuiper_lang::CompilerConfig::default(),
    ) {
        Ok(expr) => CompileResult {
            error: KuiperError {
                error: std::ptr::null_mut(),
                is_error: false,
                start: 0,
                end: 0,
            },
            result: Box::into_raw(Box::new(expr)),
        },
        Err(e) => CompileResult {
            error: e.into(),
            result: std::ptr::null_mut(),
        },
    };
    Box::into_raw(Box::new(res))
}

#[derive(Default)]
/// Opaque compiler config struct. Since the rust compiler config
/// is by-value, we need to wrap it in an option to be able to modify it through the C API.
pub struct CompilerConfig {
    inner: Option<kuiper_lang::CompilerConfig>,
}

#[no_mangle]
/// Create a new compiler configuration with default settings.
pub extern "C" fn new_compiler_config() -> *mut CompilerConfig {
    Box::into_raw(Box::new(CompilerConfig::default()))
}

#[no_mangle]
/// Destroy a compiler configuration allocated by `new_compiler_config`.
///
/// # Safety
///
/// `config` must be a valid, non-null pointer to a `CompilerConfig`,
/// typically obtained from `new_compiler_config`.
pub unsafe extern "C" fn destroy_compiler_config(config: *mut CompilerConfig) {
    unsafe { drop(Box::from_raw(config)) };
}

#[no_mangle]
/// Set the optimizer operation limit for a compiler configuration.
///
/// # Safety
///
/// `config` must be a valid, non-null pointer to a `CompilerConfig`,
/// typically obtained from `new_compiler_config`.
pub unsafe extern "C" fn config_set_optimizer_operation_limit(
    config: *mut CompilerConfig,
    limit: i64,
) {
    let config = unsafe { &mut *config };
    config.inner = Some(
        config
            .inner
            .take()
            .unwrap_or_default()
            .optimizer_operation_limit(limit),
    );
}

#[no_mangle]
/// Set the maximum number of macro expansions for a compiler configuration.
///
/// # Safety
///
/// `config` must be a valid, non-null pointer to a `CompilerConfig`,
/// typically obtained from `new_compiler_config`.
pub unsafe extern "C" fn config_set_max_macro_expansions(
    config: *mut CompilerConfig,
    limit: i32,
) -> *mut CompilerConfig {
    let config = unsafe { &mut *config };
    config.inner = Some(
        config
            .inner
            .take()
            .unwrap_or_default()
            .max_macro_expansions(limit),
    );
    config
}

#[no_mangle]
/// Add a custom function to a compiler configuration. The `implementation` function will be called
/// when the custom function is invoked in a kuiper expression. The `implementation` function should
/// return a `CustomFunctionResult` containing the result of the function or an error message.
///
/// # Safety
///
/// `config` must be a valid, non-null pointer to a `CompilerConfig`,
/// typically obtained from `new_compiler_config`.
///
/// `implementation` must be a valid function pointer that can be safely called
/// with the provided arguments.
pub unsafe extern "C" fn config_add_custom_function(
    config: *mut CompilerConfig,
    name: *const c_char,
    implementation: extern "C" fn(*const *mut c_char, usize) -> CustomFunctionResult,
) {
    let name = unsafe { CStr::from_ptr(name).to_str().unwrap() };
    let config = unsafe { &mut *config };

    config.inner = Some(
        config
            .inner
            .take()
            .unwrap_or_default()
            .with_custom_dynamic_function(
                name,
                Arc::new(CustomBuilder {
                    function: Arc::new(implementation),
                    name: name.to_string(),
                }),
            ),
    );
}

#[no_mangle]
/// Compile a kuiper expression from a string and a list of inputs, using custom compiler configuration.
///
/// Returns a result struct in which exactly one of `error` or `result` is non-null.
///
/// # Safety
///
/// `data` must be a valid, utf8-encoded, null terminated string. `inputs` must be an array of such strings
/// with length `len`. If `len` is 0, `inputs` may be null.
///
/// `config` must be a valid compiler config instance, typically obtained from `new_compiler_config`
/// and modified with the other config functions.
pub unsafe extern "C" fn compile_expression_with_config(
    data: *const c_char,
    inputs: *const *const c_char,
    len: usize,
    config: *mut CompilerConfig,
) -> *mut CompileResult {
    let config = unsafe { &mut *config };
    let r = match config.inner.as_ref() {
        Some(inner) => compile_expression_internal(data, inputs, len, inner),
        None => {
            compile_expression_internal(data, inputs, len, &kuiper_lang::CompilerConfig::default())
        }
    };
    let res = match r {
        Ok(expr) => CompileResult {
            error: KuiperError {
                error: std::ptr::null_mut(),
                is_error: false,
                start: 0,
                end: 0,
            },
            result: Box::into_raw(Box::new(expr)),
        },
        Err(e) => CompileResult {
            error: e.into(),
            result: std::ptr::null_mut(),
        },
    };
    Box::into_raw(Box::new(res))
}

#[repr(C)]
#[derive(Debug)]
pub struct CustomFunctionResult {
    pub is_error: bool,
    pub data: *mut c_char,
    pub free_data: extern "C" fn(*mut c_void),
}

#[derive(Debug)]
struct Custom {
    function: Arc<unsafe extern "C" fn(*const *mut c_char, usize) -> CustomFunctionResult>,
    args: Vec<ExpressionType>,
    span: Span,
    name: String,
}

impl Display for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(", self.name)?;
        let mut needs_comma = false;
        for arg in &self.args {
            if needs_comma {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
            needs_comma = true;
        }
        write!(f, ")")
    }
}

impl Expression for Custom {
    fn is_deterministic(&self) -> bool {
        false
    }

    fn resolve<'a>(
        &'a self,
        state: &mut kuiper_lang::ExpressionExecutionState<'a, '_>,
    ) -> Result<kuiper_lang::ResolveResult<'a>, TransformError> {
        let args = self
            .args
            .iter()
            .map(|arg| {
                let item = arg.resolve(state)?;
                let item_str = serde_json::to_string(&item.as_ref()).map_err(|e| {
                    TransformError::new_invalid_operation(e.to_string(), &self.span)
                })?;
                Ok::<_, TransformError>(item_str)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let c_args = args
            .iter()
            .map(|arg| CString::new(arg.as_str()).unwrap().into_raw())
            .collect::<Vec<_>>();
        let res = unsafe { (self.function)(c_args.as_ptr(), c_args.len()) };

        // Clean up the C strings we allocated for the arguments
        for arg in c_args {
            unsafe { drop(CString::from_raw(arg)) };
        }
        let res_str = unsafe { CStr::from_ptr(res.data) }
            .to_str()
            .map(|v| v.to_string())
            .map_err(|e| TransformError::new_invalid_operation(e.to_string(), &self.span));

        // Call the provided free function to clean up the result string
        (res.free_data)(res.data as *mut c_void);
        let res_str = res_str?;

        if res.is_error {
            Err(TransformError::new_invalid_operation(res_str, &self.span))
        } else {
            let v: serde_json::Value = serde_json::from_str(&res_str)
                .map_err(|e| TransformError::new_invalid_operation(e.to_string(), &self.span))?;
            Ok(kuiper_lang::ResolveResult::Owned(v))
        }
    }
}

impl ExpressionMeta for Custom {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.args.iter_mut())
    }
}

struct CustomBuilder {
    function: Arc<unsafe extern "C" fn(*const *mut c_char, usize) -> CustomFunctionResult>,
    name: String,
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
            name: self.name.clone(),
        }))
    }
}

#[repr(C)]
pub struct TransformResult {
    pub error: KuiperError,
    pub result: *mut c_char,
}

unsafe fn run_expression_internal(
    data: *const *const c_char,
    len: usize,
    expression: *const ExpressionType,
) -> Result<String, InteropError> {
    let data = if len > 0 {
        let data_raw = unsafe { &*slice_from_raw_parts(data, len) };
        data_raw
            .iter()
            .map(|i| CStr::from_ptr(*i).to_str())
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    unsafe {
        let data_json = data
            .into_iter()
            .map(serde_json::from_str)
            .collect::<Result<Vec<Value>, _>>()?;
        let res = (*expression).run(&data_json)?;
        Ok(res.to_string())
    }
}

/// Destroy a transform result, this is called from external code to safely dispose of results
/// allocated by `run_expression` after the result has been extracted.
///
/// # Safety
///
/// `data` must be a valid, non-null pointer to a TransformResult, typically obtained from `run_expression`.
#[no_mangle]
pub unsafe extern "C" fn destroy_transform_result(data: *mut TransformResult) {
    let data = unsafe { Box::from_raw(data) };
    if !data.error.error.is_null() {
        unsafe { drop(CString::from_raw(data.error.error)) };
    }
    if !data.result.is_null() {
        unsafe { drop(CString::from_raw(data.result)) };
    }
}

/// Convert an expression to its string representation
///
/// # Safety
///
/// `data` must be a valid pointer to an `ExpressionType`.
#[no_mangle]
pub unsafe extern "C" fn expression_to_string(data: *const ExpressionType) -> *mut c_char {
    let str = unsafe { &*data }.to_string();
    CString::new(str).unwrap().into_raw()
}

/// Destroy a string allocated by rust
///
/// # Safety
///
/// `data` must be a valid, null-terminated, UTF-8 encoded string.
/// Do not call this on strings not originally allocated by rust.
#[no_mangle]
pub unsafe extern "C" fn destroy_string(data: *mut c_char) {
    if !data.is_null() {
        drop(CString::from_raw(data))
    }
}

/// Run a kuiper expression with a list of inputs.
///
/// Returns a result struct in which exactly one of `error` or `result` is non-null.
///
/// # Safety
///
/// `data` must be an array of valid, utf8-encoded, null-terminated strings
/// with length `len`. If `len` is 0, `data` may be null.
///
/// `expression` must be a valid pointer to an `ExpressionType`, typically obtained from
/// `compile_expression` and `get_expression_from_compile_result`
#[no_mangle]
pub unsafe extern "C" fn run_expression(
    data: *const *const c_char,
    len: usize,
    expression: *const ExpressionType,
) -> *mut TransformResult {
    let res = match run_expression_internal(data, len, expression) {
        Ok(expr) => TransformResult {
            error: KuiperError {
                error: std::ptr::null_mut(),
                is_error: false,
                start: 0,
                end: 0,
            },
            result: CString::new(expr).unwrap().into_raw(),
        },
        Err(e) => TransformResult {
            error: e.into(),
            result: std::ptr::null_mut(),
        },
    };
    Box::into_raw(Box::new(res))
}
