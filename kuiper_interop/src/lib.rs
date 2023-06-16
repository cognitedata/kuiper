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
    let inputs_raw = unsafe { &*slice_from_raw_parts(inputs, len) };
    let inputs: Vec<_> = inputs_raw
        .into_iter()
        .map(|i| CStr::from_ptr(*i).to_str())
        .collect::<Result<Vec<_>, _>>()?;

    Ok(kuiper_lang::compile_expression(data.to_str()?, &inputs)?)
}

#[no_mangle]
pub unsafe extern "C" fn destroy_compile_result(data: *mut CompileResult) {
    let data = unsafe { Box::from_raw(data) };
    if !data.error.is_null() {
        unsafe { Box::from_raw(data.error) };
    }
    if !data.result.is_null() {
        unsafe { Box::from_raw(data.result) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn destroy_expression(data: *mut ExpressionType) {
    unsafe { Box::from_raw(data) };
}

#[no_mangle]
pub unsafe extern "C" fn get_expression_from_compile_result(
    data: *mut CompileResult,
) -> *mut ExpressionType {
    let data = unsafe { Box::from_raw(data) };
    if !data.error.is_null() {
        unsafe { Box::from_raw(data.error) };
    }
    data.result
}

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
    let data_raw = unsafe { &*slice_from_raw_parts(data, len) };
    let data: Vec<_> = data_raw
        .into_iter()
        .map(|i| CStr::from_ptr(*i).to_str())
        .collect::<Result<Vec<_>, _>>()?;
    unsafe {
        let data_json = data
            .into_iter()
            .map(serde_json::from_str)
            .collect::<Result<Vec<Value>, _>>()?;
        let res = (*expression).run(&data_json)?;
        Ok(res.to_string())
    }
}

#[no_mangle]
pub unsafe extern "C" fn destroy_transform_result(data: *mut TransformResult) {
    let data = unsafe { Box::from_raw(data) };
    if !data.error.is_null() {
        unsafe { Box::from_raw(data.error) };
    }
    if !data.result.is_null() {
        unsafe { Box::from_raw(data.result) };
    }
}

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
