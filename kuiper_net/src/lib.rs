#![allow(improper_ctypes_definitions)]
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    ptr::null_mut,
};

use interoptopus::{
    extra_type, ffi_function, ffi_service, ffi_service_ctor, ffi_service_method, ffi_type,
    function, pattern,
    patterns::{option::FFIOption, result::FFIError, slice::FFISlice, string::AsciiPointer},
    Error, Inventory, InventoryBuilder,
};
use kuiper_lang::ExpressionType;

#[derive(Debug)]
#[ffi_type(patterns(ffi_error))]
#[repr(C)]
pub enum KuiperFFIError {
    SUCCESS = 1,
    NULL = 2,
    PANIC = 3,
}

impl FFIError for KuiperFFIError {
    const SUCCESS: Self = Self::SUCCESS;

    const NULL: Self = Self::NULL;

    const PANIC: Self = Self::PANIC;
}

#[ffi_type]
#[repr(C)]
pub struct CompileResult {
    pub error: FFIOption<KuiperError>,
    pub result: FFIOption<KuiperExpression>,
}

impl From<Error> for CompileResult {
    fn from(err: Error) -> Self {
        println!("Return error: {err:?}");
        Self {
            error: FFIOption::some(KuiperError::from(err)),
            result: FFIOption::none(),
        }
    }
}

#[ffi_type]
#[repr(C)]
pub struct ExecuteResult {
    pub error: FFIOption<KuiperError>,
    pub result: FFIOption<KuiperExpressionResult>,
}

impl From<Error> for ExecuteResult {
    fn from(err: Error) -> Self {
        Self {
            error: FFIOption::some(KuiperError::from(err)),
            result: FFIOption::none(),
        }
    }
}

#[ffi_type(opaque)]
#[repr(C)]
pub struct KuiperError {
    err: CString,
}

impl Default for KuiperError {
    fn default() -> Self {
        Self {
            err: Default::default(),
        }
    }
}

#[ffi_service(error = "KuiperFFIError", prefix = "kuiper_error_")]
impl KuiperError {
    #[ffi_service_ctor]
    pub fn new() -> Result<Self, KuiperFFIError> {
        Ok(Self {
            err: CString::default(),
        })
    }

    #[ffi_service_method(on_panic = "return_default")]
    pub fn error(&self) -> AsciiPointer {
        AsciiPointer::from_cstr(&self.err)
    }
}

impl<T> From<T> for KuiperError
where
    T: ToString,
{
    fn from(value: T) -> Self {
        Self {
            err: CString::new(value.to_string()).unwrap(),
        }
    }
}

#[ffi_type(opaque)]
#[repr(C)]
pub struct InputStringWrapper {
    data: *mut String,
}

impl Default for InputStringWrapper {
    fn default() -> Self {
        Self { data: null_mut() }
    }
}

#[ffi_service(error = "KuiperFFIError", prefix = "kuiper_input_string_wrapper_")]
impl InputStringWrapper {
    #[ffi_service_ctor]
    pub fn new(data: AsciiPointer) -> Result<Self, KuiperFFIError> {
        let str = Box::leak(Box::new(data.as_str().unwrap().to_string()));
        println!("Input data: {:?} {}", str.as_ptr(), str);
        Ok(Self { data: str })
    }
}

#[derive(Default)]
#[ffi_type(opaque)]
#[repr(C)]
pub struct KuiperExpressionResult {
    data: CString,
}

#[ffi_service(error = "KuiperFFIError", prefix = "kuiper_expression_result_")]
impl KuiperExpressionResult {
    // For some dumb reason all services must have a constructor. Even if that makes no sense.
    #[ffi_service_ctor]
    pub fn new() -> Result<Self, KuiperFFIError> {
        Ok(Self {
            data: CString::default(),
        })
    }

    #[ffi_service_method(on_panic = "return_default")]
    pub fn data(&self) -> AsciiPointer {
        AsciiPointer::from_cstr(&self.data)
    }
}

#[ffi_type(opaque)]
#[repr(C)]
pub struct KuiperExpression {
    expr: *mut ExpressionType,
}

impl Default for KuiperExpression {
    fn default() -> Self {
        Self { expr: null_mut() }
    }
}

#[ffi_service(error = "KuiperFFIError", prefix = "kuiper_expression_")]
impl KuiperExpression {
    #[ffi_service_ctor]
    pub fn new() -> Result<Self, KuiperFFIError> {
        Ok(Self { expr: null_mut() })
    }

    #[ffi_service_method(on_panic = "undefined_behavior")]
    pub fn execute(&self, data: FFISlice<InputStringWrapper>) -> ExecuteResult {
        let expr = unsafe { &*self.expr };

        let mut inputs = vec![];
        for val in data.iter() {
            let value = match serde_json::from_str(unsafe { &*val.data }) {
                Ok(x) => x,
                Err(e) => {
                    return ExecuteResult {
                        error: FFIOption::some(KuiperError::from(e)),
                        result: FFIOption::none(),
                    }
                }
            };
            inputs.push(value);
        }

        let res = expr.run(inputs.iter(), "net_exec");
        match res {
            Ok(x) => ExecuteResult {
                result: FFIOption::some(KuiperExpressionResult {
                    data: CString::new(serde_json::to_string(&x.into_owned()).unwrap()).unwrap(),
                }),
                error: FFIOption::none(),
            },
            Err(e) => ExecuteResult {
                error: FFIOption::some(KuiperError::from(e)),
                result: FFIOption::none(),
            },
        }
    }
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn compile_expression(
    data: AsciiPointer,
    inputs: FFISlice<*mut InputStringWrapper>,
) -> CompileResult {
    println!("Call compile expression");
    let data_string = match data.as_str() {
        Ok(x) => x,
        Err(e) => {
            return CompileResult::from(e);
        }
    };
    println!("Build inputs: {}, {:?}", inputs.len(), inputs.as_ptr());
    let mut inputs_dat = HashMap::new();
    println!("Begin build inputs: {}", inputs.len());
    for (idx, inp) in inputs.iter().enumerate() {
        println!("Get ptr: {:?}", unsafe { (*(*inp)).data });
        inputs_dat.insert(unsafe { &*(*(*inp)).data }.clone(), idx);
    }
    let compile_res = kuiper_lang::compile_expression(data_string, &mut inputs_dat, "net");
    println!("Return some result");
    match compile_res {
        Ok(x) => CompileResult {
            error: FFIOption::none(),
            result: FFIOption::some(KuiperExpression {
                expr: Box::leak(Box::new(x)),
            }),
        },
        Err(e) => CompileResult {
            error: FFIOption::some(KuiperError::from(e)),
            result: FFIOption::none(),
        },
    }
}

pub fn ffi_inventory() -> Inventory {
    InventoryBuilder::new()
        .register(function!(compile_expression))
        .register(pattern!(KuiperExpression))
        .register(pattern!(KuiperExpressionResult))
        .register(pattern!(KuiperError))
        .register(pattern!(InputStringWrapper))
        .register(extra_type!(ExecuteResult))
        .register(extra_type!(CompileResult))
        .inventory()
}
