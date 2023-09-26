use std::{
    ffi::{c_char, CStr, CString},
    ptr::slice_from_raw_parts,
};

use kuiper_lang::{CompileError, ExpressionType, TransformError};
use serde_json::Value;
use thiserror::Error;

#[repr(C)]
pub struct CompileResult {
    pub error: *mut c_char,
    pub result: *mut ExpressionType,
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
    data: *mut c_char,
    inputs: *mut *mut c_char,
    len: usize,
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

    Ok(kuiper_lang::compile_expression(data.to_str()?, &inputs)?)
}

/// Destroy a compile result. Called from external code to correctly free rust allocated memory.
///
/// # Safety
///
/// `data` must be a valid, non-null rust pointer to a `CompileResult`, typically produced by `compile_expression`
#[no_mangle]
pub unsafe extern "C" fn destroy_compile_result(data: *mut CompileResult) {
    let data = unsafe { Box::from_raw(data) };
    if !data.error.is_null() {
        unsafe { std::mem::drop(CString::from_raw(data.error)) };
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
    if !data.error.is_null() {
        unsafe { drop(Box::from_raw(data.error)) };
    }
    data.result
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
    data: *mut c_char,
    inputs: *mut *mut c_char,
    len: usize,
) -> *mut CompileResult {
    let res = match compile_expression_internal(data, inputs, len) {
        Ok(expr) => CompileResult {
            error: std::ptr::null_mut(),
            result: Box::leak(Box::new(expr)),
        },
        Err(e) => CompileResult {
            error: CString::new(e.to_string()).unwrap().into_raw(),
            result: std::ptr::null_mut(),
        },
    };
    Box::leak(Box::new(res))
}

pub struct TransformResult {
    pub error: *mut c_char,
    pub result: *mut c_char,
}

unsafe fn run_expression_internal(
    data: *mut *mut c_char,
    len: usize,
    expression: *mut ExpressionType,
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
    if !data.error.is_null() {
        unsafe { drop(CString::from_raw(data.error)) };
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
pub unsafe extern "C" fn expression_to_string(data: *mut ExpressionType) -> *mut c_char {
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
    data: *mut *mut c_char,
    len: usize,
    expression: *mut ExpressionType,
) -> *mut TransformResult {
    let res = match run_expression_internal(data, len, expression) {
        Ok(expr) => TransformResult {
            error: std::ptr::null_mut(),
            result: CString::new(expr).unwrap().into_raw(),
        },
        Err(e) => TransformResult {
            error: CString::new(e.to_string()).unwrap().into_raw(),
            result: std::ptr::null_mut(),
        },
    };
    Box::leak(Box::new(res))
}
