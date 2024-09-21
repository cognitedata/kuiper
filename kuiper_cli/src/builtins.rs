// This file is automatically created by kuiper_documentation/codegen.py. Do not edit it directly.
//
// To change the content of this file, edit kuiper_documentation/functions.yaml instead.

use lazy_static::lazy_static;
use std::collections::HashMap;

pub const BUILT_INS: [&str; 47] = [
    "all(",
    "any(",
    "atan2(",
    "case(",
    "ceil(",
    "chars(",
    "chunk(",
    "coalesce(",
    "concat(",
    "contains(",
    "digest(",
    "distinct_by(",
    "except(",
    "filter(",
    "flatmap(",
    "float(",
    "floor(",
    "format_timestamp(",
    "if(",
    "int(",
    "join(",
    "length(",
    "log(",
    "map(",
    "max(",
    "min(",
    "now(",
    "pairs(",
    "pow(",
    "reduce(",
    "replace(",
    "round(",
    "select(",
    "slice(",
    "split(",
    "string(",
    "string_join(",
    "substring(",
    "sum(",
    "tail(",
    "to_object(",
    "to_unix_timestamp(",
    "trim_whitespace(",
    "try_bool(",
    "try_float(",
    "try_int(",
    "zip(",
];

pub struct FunctionDef {
    pub signature: &'static str,
    pub description: &'static str,
}

lazy_static! {
    pub static ref HELP: HashMap<&'static str, FunctionDef> = HashMap::from([

        (
            "all",
            FunctionDef {
                signature: "all(x)",
                description: "Returns true if all items in the array `x` is true.",
            }
        ),
        (
            "any",
            FunctionDef {
                signature: "any(x)",
                description: "Returns true if any items in the array `x` is true.",
            }
        ),
        (
            "atan2",
            FunctionDef {
                signature: "atan2(x, y)",
                description: "Returns the inverse tangent of `x`/`y` in radians between -pi and pi.",
            }
        ),
        (
            "case",
            FunctionDef {
                signature: "case(x, c1, r1, c2, r2, ..., (default))",
                description: "Compare `x` to each of `c1`, `c2`, etc. and return the matching `r1`, `r2` of the first match. If no entry matches, a final optional expression can be returned as default.",
            }
        ),
        (
            "ceil",
            FunctionDef {
                signature: "ceil(x)",
                description: "Returns `x` rounded up to the nearest integer.",
            }
        ),
        (
            "chars",
            FunctionDef {
                signature: "chars(x)",
                description: "Creates an array of characters from a string.",
            }
        ),
        (
            "chunk",
            FunctionDef {
                signature: "chunk(x, s)",
                description: "Converts the list `x` into several lists of length at most `s`",
            }
        ),
        (
            "coalesce",
            FunctionDef {
                signature: "coalesce(a, b, ...)",
                description: "Return the first non-null value in the list of values.",
            }
        ),
        (
            "concat",
            FunctionDef {
                signature: "concat(x, y, ...)",
                description: "Concatenate any number of strings.",
            }
        ),
        (
            "contains",
            FunctionDef {
                signature: "conatins(x, a)",
                description: "Returns true if the array `x` contains item `a`.",
            }
        ),
        (
            "digest",
            FunctionDef {
                signature: "digest(a, b, ...)",
                description: "Compute the SHA256 hash of the list of values.",
            }
        ),
        (
            "distinct_by",
            FunctionDef {
                signature: "distinct_by(x, (a(, b)) => ...)",
                description: "Returns a list or object where the elements are distinct by the returned value of the given lambda function. The lambda function either takes list values, or object (value, key) pairs.",
            }
        ),
        (
            "except",
            FunctionDef {
                signature: "except(x, (v(, k)) => ...)` or `except(x, l)",
                description: "Returns a list or object where keys or entries maching the predicate have been removed.
If the second argument is a lambda, it will be given the entry and if it returns `true`, the entry is removed.
If the second argument is a list, any entry also found in this list will be removed.",
            }
        ),
        (
            "filter",
            FunctionDef {
                signature: "filter(x, it => ...)",
                description: "Removes any item from the list `x` where the lambda function returns `false` or `null`.",
            }
        ),
        (
            "flatmap",
            FunctionDef {
                signature: "flatmap(x, it => ...)",
                description: "Applies the lambda function to every item in the list `x` and flattens the result.

For example, if the lambda function returns a list, the result of the `flatmap` will just be a list instead of a list of lists.",
            }
        ),
        (
            "float",
            FunctionDef {
                signature: "float(x)",
                description: "Converts `x` into a floating point number if possible. If the conversion fails, the whole mapping will fail.

Consider using [try_float](#try_float) instead if you need error handling.",
            }
        ),
        (
            "floor",
            FunctionDef {
                signature: "floor(x)",
                description: "Returns `x` rounded down to the nearest integer.",
            }
        ),
        (
            "format_timestamp",
            FunctionDef {
                signature: "format_timestamp(x, f)",
                description: "Converts the Unix timestamp `x` into a string representation based on the format `f`.

The format is given using the table found [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).",
            }
        ),
        (
            "if",
            FunctionDef {
                signature: "if(x, y, (z))",
                description: "Returns `y` if `x` evaluates to `true`, otherwise return `z`, or `null` if `z` is omitted.",
            }
        ),
        (
            "int",
            FunctionDef {
                signature: "int(x)",
                description: "Converts `x` into an integer if possible. If the conversion fails, the whole mapping will fail.

Consider using [try_int](#try_int) instead if you need error handling.",
            }
        ),
        (
            "join",
            FunctionDef {
                signature: "join(a, b, ...)",
                description: "Returns the union of the given objects or arrays. If a key is present in multiple objects, they are overwritten by later objects. Arrays are simply merged.",
            }
        ),
        (
            "length",
            FunctionDef {
                signature: "length(x)",
                description: "Returns the length on the list, string or object `x`.",
            }
        ),
        (
            "log",
            FunctionDef {
                signature: "log(x, y)",
                description: "Returns the base `y` logarithm of `x`.",
            }
        ),
        (
            "map",
            FunctionDef {
                signature: "map(x, (it(, index)) => ...)",
                description: "Applies the lambda function to every item in the list `x`. The lambda takes an optional second input which is the index of the item in the list.

If applied to an object, the first input is the value, and the second is the key. The result is the new value.",
            }
        ),
        (
            "max",
            FunctionDef {
                signature: "max(a, b)",
                description: "Returns the larger of the two numbers `a` and `b`.",
            }
        ),
        (
            "min",
            FunctionDef {
                signature: "min(a, b)",
                description: "Returns the smaller of the two numbers `a` and `b`.",
            }
        ),
        (
            "now",
            FunctionDef {
                signature: "now()",
                description: "Returns the current time as a millisecond Unix timestamp, that is, the number of milliseconds since midnight 1/1/1970 UTC.",
            }
        ),
        (
            "pairs",
            FunctionDef {
                signature: "pairs(x)",
                description: "Convert the object `x` into a list of key/value pairs.",
            }
        ),
        (
            "pow",
            FunctionDef {
                signature: "pow(x, y)",
                description: "Returns `x` to the power of `y`",
            }
        ),
        (
            "reduce",
            FunctionDef {
                signature: "reduce(x, (acc, val) => ..., init)",
                description: "Returns the value obtained by reducing the list `x`. The lambda function is called once for each element in the list `val`, and the returned value is passed as `acc` in the next iteration. The `init` will be given as the initial `acc` for the first call to the lambda function.",
            }
        ),
        (
            "replace",
            FunctionDef {
                signature: "replace(a, b, c)",
                description: "Replaces a string with another string",
            }
        ),
        (
            "round",
            FunctionDef {
                signature: "round(x)",
                description: "Returns `x` rounded to the nearest integer.",
            }
        ),
        (
            "select",
            FunctionDef {
                signature: "select(x, (v(, k)) => ...)` or `select(x, [1, 2, 3])",
                description: "Returs a list or object where the lambda returns true. If the second argument is a list, the list values or object keys found in that list are used to select from the source.",
            }
        ),
        (
            "slice",
            FunctionDef {
                signature: "slice(x, start(, end))",
                description: "Creates a sub-array from an array `x` from `start` to `end`. If `end is not specified, go from `start` the end of the array. If `start` or `end` are negative, count from the end of the array.",
            }
        ),
        (
            "split",
            FunctionDef {
                signature: "split(a, b)",
                description: "Splits string `a` on any occurences of `b`. If `b` is an empty string, this will split on each character, including before the first and after the last.",
            }
        ),
        (
            "string",
            FunctionDef {
                signature: "string(x)",
                description: "Converts `x` into a string.

`null`s will be converted into empty strings.",
            }
        ),
        (
            "string_join",
            FunctionDef {
                signature: "string_join(x(, a))",
                description: "Returns a string with all the elements of `x`, separated by `a`. If `a` is omitted, the strings will be joined without any separator.",
            }
        ),
        (
            "substring",
            FunctionDef {
                signature: "substring(x, start(, end))",
                description: "Creates a substring of an input string `x` from `start` to `end`. If `end` is not specified, go from `start` to end of string. If `start` or `end` are negative, count from the end of the string.",
            }
        ),
        (
            "sum",
            FunctionDef {
                signature: "sum(x)",
                description: "Sums the numbers in the array `x`.",
            }
        ),
        (
            "tail",
            FunctionDef {
                signature: "tail(x(, n))",
                description: "Takes the last element of the list `x`. If `n` is given, takes the last `n` elements, and returns a list if `n` > 1.",
            }
        ),
        (
            "to_object",
            FunctionDef {
                signature: "to_object(x, val => ...(, val => ...))",
                description: "Converts the array `x` into an object by producing the key and value from two lambdas.

The first lambda produces the key, and the second (optional) produces the value. If the second is
left out, the input is used as a value directly.",
            }
        ),
        (
            "to_unix_timestamp",
            FunctionDef {
                signature: "to_unix_timestamp(x, f)",
                description: "Converts the string `x` into a millisecond Unix timestamp using the format string `f`.

The format is given using the table found [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).",
            }
        ),
        (
            "trim_whitespace",
            FunctionDef {
                signature: "trim_whitespace(x)",
                description: "Removes any whitespace from the start and end of `x`",
            }
        ),
        (
            "try_bool",
            FunctionDef {
                signature: "try_bool(a, b)",
                description: "Try convert `a` to a boolean, if it fails, return `b`.",
            }
        ),
        (
            "try_float",
            FunctionDef {
                signature: "try_float(a, b)",
                description: "Try convert `a` to a float, if it fails, return `b`.",
            }
        ),
        (
            "try_int",
            FunctionDef {
                signature: "try_int(a, b)",
                description: "Try convert `a` to a int, if it fails, return `b`.",
            }
        ),
        (
            "zip",
            FunctionDef {
                signature: "zip(x, y, ..., (i1, i2, ...) => ...)",
                description: "Takes a number of arrays, call the given lambda function on each entry, and return a single array from the result of each call. The returned array will be as long as the longest argument, null will be given for the shorter input arrays when they run out.",
            }
        ),
    ]);
}
