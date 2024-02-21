// This file is automatically created by kuiper_documentation/codegen.py. Do not edit it directly.
//
// To change the content of this file, edit kuiper_documentation/functions.yaml instead.

export type KuiperInput = {
    label: string,
    description: string,
};

export const builtIns: KuiperInput[] = [
    { label: "atan2", description: "`atan2(x, y)`: Returns the inverse tangent of `x`/`y` in radians between -pi and pi." },
    { label: "case", description: "`case(x, c1, r1, c2, r2, ..., (default))`: Compare `x` to each of `c1`, `c2`, etc. and return the matching `r1`, `r2` of the first match. If no entry matches, a final optional expression can be returned as default." },
    { label: "ceil", description: "`ceil(x)`: Returns `x` rounded up to the nearest integer." },
    { label: "chars", description: "`chars(x)`: Creates an array of characters from a string." },
    { label: "chunk", description: "`chunk(x, s)`: Converts the list `x` into several lists of length at most `s`" },
    { label: "concat", description: "`concat(x, y, ...)`: Concatenate any number of strings." },
    { label: "distinct_by", description: "`distinct_by(x, (a(, b)) => ...)`: Returns a list or object where the elements are distinct by the returned value of the given lambda function. The lambda function either takes list values, or object (value, key) pairs." },
    { label: "except", description: "`except(x, (v(, k)) => ...)` or `except(x, l)`: Returns a list or object where keys or entries maching the predicate have been removed." },
    { label: "filter", description: "`filter(x, it => ...)`: Removes any item from the list `x` where the lambda function returns `false` or `null`." },
    { label: "flatmap", description: "`flatmap(x, it => ...)`: Applies the lambda function to every item in the list `x` and flattens the result." },
    { label: "float", description: "`float(x)`: Converts `x` into a floating point number if possible. If the conversion fails, the whole mapping will fail." },
    { label: "floor", description: "`floor(x)`: Returns `x` rounded down to the nearest integer." },
    { label: "format_timestamp", description: "`format_timestamp(x, f)`: Converts the Unix timestamp `x` into a string representation based on the format `f`." },
    { label: "if", description: "`if(x, y, (z))`: Returns `y` if `x` evaluates to `true`, otherwise return `z`, or `null` if `z` is omitted." },
    { label: "int", description: "`int(x)`: Converts `x` into an integer if possible. If the conversion fails, the whole mapping will fail." },
    { label: "join", description: "`join(a, b)`: Returns the union of the two objects `a` and `b`. If a key is present in both objects, `b` takes precedent." },
    { label: "length", description: "`length(x)`: Returns the length on the list, string or object `x`." },
    { label: "log", description: "`log(x, y)`: Returns the base `y` logarithm of `x`." },
    { label: "map", description: "`map(x, (it(, index)) => ...)`: Applies the lambda function to every item in the list `x`. The lambda takes an optional second input which is the index of the item in the list." },
    { label: "now", description: "`now()`: Returns the current time as a millisecond Unix timestamp, that is, the number of milliseconds since midnight 1/1/1970 UTC." },
    { label: "pairs", description: "`pairs(x)`: Convert the object `x` into a list of key/value pairs." },
    { label: "pow", description: "`pow(x, y)`: Returns `x` to the power of `y`" },
    { label: "reduce", description: "`reduce(x, (acc, val) => ..., init)`: Returns the value obtained by reducing the list `x`. The lambda function is called once for each element in the list `val`, and the returned value is passed as `acc` in the next iteration. The `init` will be given as the initial `acc` for the first call to the lambda function." },
    { label: "replace", description: "`replace(a, b, c)`: Replaces a string with another string" },
    { label: "round", description: "`round(x)`: Returns `x` rounded to the nearest integer." },
    { label: "select", description: "`select(x, (v(, k)) => ...)` or `select(x, [1, 2, 3])`: Returs a list or object where the lambda returns true. If the second argument is a list, the list values or object keys found in that list are used to select from the source." },
    { label: "slice", description: "`slice(x, start(, end))`: Creates a sub-array from an array `x` from `start` to `end`. If `end is not specified, go from `start` the end of the array. If `start` or `end` are negative, count from the end of the array." },
    { label: "split", description: "`split(a, b)`: Splits string `a` on any occurences of `b`. If `b` is an empty string, this will split on each character, including before the first and after the last." },
    { label: "string", description: "`string(x)`: Converts `x` into a string." },
    { label: "substring", description: "`substring(x, start(, end))`: Creates a substring of an input string `x` from `start` to `end`. If `end` is not specified, go from `start` to end of string. If `start` or `end` are negative, count from the end of the string." },
    { label: "tail", description: "`tail(x(, n))`: Takes the last element of the list `x`. If `n` is given, takes the last `n` elements, and returns a list if `n` > 1." },
    { label: "to_object", description: "`to_object(x, val => ...(, val => ...))`: Converts the array `x` into an object by producing the key and value from two lambdas." },
    { label: "to_unix_timestamp", description: "`to_unix_timestamp(x, f)`: Converts the string `x` into a millisecond Unix timestamp using the format string `f`." },
    { label: "trim_whitespace", description: "`trim_whitespace(x)`: Removes any whitespace from the start and end of `x`" },
    { label: "try_bool", description: "`try_bool(a, b)`: Try convert `a` to a boolean, if it fails, return `b`." },
    { label: "try_float", description: "`try_float(a, b)`: Try convert `a` to a float, if it fails, return `b`." },
    { label: "try_int", description: "`try_int(a, b)`: Try convert `a` to a int, if it fails, return `b`." },
    { label: "zip", description: "`zip(x, y, ..., (i1, i2, ...) => ...)`: Takes a number of arrays, call the given lambda function on each entry, and return a single array from the result of each call. The returned array will be as long as the longest argument, null will be given for the shorter input arrays when they run out." },
];
