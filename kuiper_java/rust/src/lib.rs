use std::ptr;

use jni::{
    objects::{JClass, JObject, JObjectArray, JString},
    sys::{jlong, jstring},
    JNIEnv,
};
use kuiper_lang::ExpressionType;
use serde_json::Value;

#[no_mangle]
#[allow(non_snake_case, reason = "JNI names")]
pub extern "system" fn Java_com_cognite_kuiper_Kuiper_compile_1expression<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
    known_inputs: JObjectArray<'local>,
) -> jlong {
    let Ok(input) = env.get_string(&input) else {
        let _ = env.throw_new(
            "com/cognite/kuiper/KuiperException",
            "Failed to get java string",
        );
        return 0;
    };

    let Ok(input) = input.to_str() else {
        let _ = env.throw_new(
            "com/cognite/kuiper/KuiperException",
            "Failed to parse java string to utf-8",
        );
        return 0;
    };

    let Ok(len) = env.get_array_length(&known_inputs) else {
        let _ = env.throw_new(
            "com/cognite/kuiper/KuiperException",
            "Failed to get inputs array length",
        );
        return 0;
    };

    let mut inputs = Vec::new();
    for i in 0..len {
        let Ok(obj) = env.get_object_array_element(&known_inputs, i) else {
            let _ = env.throw_new(
                "com/cognite/kuiper/KuiperException",
                format!("Failed to get inputs array element {i}"),
            );
            return 0;
        };
        let str = obj.into();
        let Ok(v) = env.get_string(&str) else {
            let _ = env.throw_new(
                "com/cognite/kuiper/KuiperException",
                "Failed to get java string",
            );
            return 0;
        };

        let Ok(inp) = v.to_str() else {
            let _ = env.throw_new(
                "com/cognite/kuiper/KuiperException",
                "Failed to parse java string to utf-8",
            );
            return 0;
        };
        inputs.push(inp.to_owned());
    }

    let inputs_ref: Vec<_> = inputs.iter().map(|v| v.as_str()).collect();

    match kuiper_lang::compile_expression(input, &inputs_ref) {
        Ok(r) => Box::leak(Box::new(r)) as *mut _ as i64,
        Err(e) => {
            // let span = e.span().unwrap_or_else(|| Range { start: 0, end: 0 });
            let _ = env.throw_new("com/cognite/kuiper/KuiperException", e.to_string());
            0
        }
    }
}

#[no_mangle]
#[allow(non_snake_case, reason = "JNI names")]
/// Run a kuiper expression, called from JNI.
///
/// # Safety
///
/// Do not call this method, it must be linked from JNI.
pub unsafe extern "system" fn Java_com_cognite_kuiper_Kuiper_run_1expression<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    expression: jlong,
    inputs: JObjectArray<'local>,
) -> jstring {
    if expression == 0 {
        let _ = env.throw_new(
            "com/cognite/kuiper/KuiperException",
            "Passed null kuiper expression",
        );

        return JObject::null().into_raw();
    }

    let Ok(len) = env.get_array_length(&inputs) else {
        let _ = env.throw_new(
            "com/cognite/kuiper/KuiperException",
            "Failed to get inputs array length",
        );
        return JObject::null().into_raw();
    };

    let mut final_inputs = Vec::new();
    for i in 0..len {
        let Ok(obj) = env.get_object_array_element(&inputs, i) else {
            let _ = env.throw_new(
                "com/cognite/kuiper/KuiperException",
                format!("Failed to get inputs array element {i}"),
            );
            return JObject::null().into_raw();
        };
        let str = obj.into();
        let Ok(v) = env.get_string(&str) else {
            let _ = env.throw_new(
                "com/cognite/kuiper/KuiperException",
                "Failed to get java string",
            );
            return JObject::null().into_raw();
        };

        let Ok(inp) = v.to_str() else {
            let _ = env.throw_new(
                "com/cognite/kuiper/KuiperException",
                "Failed to parse java string to utf-8",
            );
            return JObject::null().into_raw();
        };
        let value: Value = match serde_json::from_str(inp) {
            Ok(r) => r,
            Err(e) => {
                let _ = env.throw_new(
                    "com/cognite/kuiper/KuiperException",
                    format!("Input is not valid JSON: {e}"),
                );
                return JObject::null().into_raw();
            }
        };

        final_inputs.push(value);
    }

    // SAFETY: No way for us to do any further checks here, if java passes us
    // something that isn't a pointer, we'll pass them a segfault right back.
    let expr = unsafe { &*(expression as *const ExpressionType) };
    let r = match expr.run(final_inputs.iter()) {
        Ok(r) => r,
        Err(e) => {
            let _ = env.throw_new("com/cognite/kuiper/KuiperException", format!("{e}"));
            return JObject::null().into_raw();
        }
    };
    let out = match serde_json::to_string(r.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            let _ = env.throw_new("com/cognite/kuiper/KuiperException", format!("{e}"));
            return JObject::null().into_raw();
        }
    };
    let Ok(r) = env.new_string(out) else {
        let _ = env.throw_new(
            "com/cognite/kuiper/KuiperException",
            "Failed to create string for result",
        );
        return JObject::null().into_raw();
    };

    r.into_raw()
}

#[no_mangle]
#[allow(non_snake_case, reason = "JNI names")]
/// Destroy a kuiper expression.
///
/// # Safety
///
/// Do not call this method, called from JNI. `expression` must be a
/// valid pointer allocated by `...compile_1expression`
pub unsafe extern "system" fn Java_com_cognite_kuiper_Kuiper_free_1expression<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    expression: jlong,
) {
    if expression == 0 {
        let _ = env.throw_new(
            "com/cognite/kuiper/KuiperException",
            "Passed null kuiper expression",
        );

        return;
    }

    unsafe {
        ptr::drop_in_place(expression as *mut ExpressionType);
    }
}
