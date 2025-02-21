#![cfg_attr(feature = "nightly", no_main)]

#[cfg(not(feature = "nightly"))]
fn main() {}

#[cfg(feature = "nightly")]
mod inner {
    use std::sync::LazyLock;

    use kuiper_lang::{compile_expression, CompileError};
    use libfuzzer_sys::fuzz_target;
    use serde_json::{json, Value};

    static DATA: LazyLock<Value> = LazyLock::new(|| {
        json!({
            "foo": "bar"
        })
    });

    fn run_expression(expr: &str) -> Result<(), CompileError> {
        let expr = compile_expression(expr, &["input"])?;

        let _ = expr.run([&*DATA]);
        Ok(())
    }
}

#[cfg(feature = "nightly")]
fuzz_target!(|data: &str| {
    // fuzzed code goes here
    let _ = inner::run_expression(data);
});
