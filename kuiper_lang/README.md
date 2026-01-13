# The Kuiper language

This crate is the main entrypoint for anything relating to the Kuiper
transformation language. It provides the `compile_expression` function along
with a few other variants.

The language itself is documented [here](https://docs.cognite.com/cdf/integration/guides/extraction/hosted_extractors/kuiper_concepts).

## Usage

```rust
use kuiper_lang::compile_expression;
use serde_json::json;

let expr = compile_expression("input.test + 5", &["input"])?;
let result = expr.run([&json!({
    "test": 3,
})])?;
assert_eq!(result.as_ref(), &json!(8));
```

### Features

 - `completions` enables collecting information for auto-completing keywords when running expressions. Note that enabling this feature incurs a small performance cost on all expression executions.

## Language Features

- **Operators**, `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, `>`, `<`, `&&`, `||` with precendence taken from the C++ standard.
- **Arrays**, `[1, 2, "test", 123.123, [123, 2]]`
- **Objects**, `{ "test": "123", concat("test", "test"): 321 }`
- **Built in functions**, like `map`, `float`, `concat`, etc. Either `pow(base, exp) or base.pow(exp)`
- **Functors**, `map` is a functor, meaning it accepts a lambda: `map(arr, field => ...)` or `arr.map(field => ...)`
- **Selector expressions**, `[1, 2, 3][1] == 2`, `input.field.value["dynamic"]`, etc.
- **Macros**, `#my_macro := (a, b) => a + b; my_macro(1, 2)`

## The `test_files` directory

The `test_files` directory contains a collection of expressions that are known
to be good. The `run_compile_tests` test in [`src/lib.rs`](./src/lib.rs) will
traverse (recursively) through this directory, collect all `.kp` files, and
compile them. This is to detect possible regressions in the language.

A file can optionally have a top comment which instructs the test runner on how
to compile (and optionally run) the expression. This comment has to be the first
token in the file, and it also has to be a single comment. Meaning if you want
multiple lines of configuration, you must use a `/* block comment */` instead
multiple `// single line comments`.

The config is in JSON, and is deserialized into these structs:

```rust
struct TestRunConfig {
    /// List of input parameters for this test run
    inputs: Vec<serde_json::Value>,
    /// The expected output
    expected: serde_json::Value,
}

struct TestCaseConfig {
    /// List of input variable names
    pub inputs: Vec<String>,
    /// List of input/output pairs to test with
    pub cases: Option<Vec<TestRunConfig>>,
}
```

See the [`input_map.kp`](./test_files/input_map.kp) file for a simple example
that defines what input variables to compile with, or the
[`max.kp`](./test_files/max.kp) file for an example that defines a test case to
execute.
