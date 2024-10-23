// This file is automatically created by kuiper_documentation/codegen.py. Do not edit it directly.
//
// To change the content of this file, edit kuiper_documentation/functions.yaml instead.

pub struct MethodDoc {
    signature: &'static str,
    documentation: &'static str,
    examples: &'static [&'static str],
}

impl MethodDoc {
    fn new(
        signature: &'static str,
        documentation: &'static str,
        examples: &'static [&'static str],
    ) -> Self {
        Self {
            signature,
            documentation,
            examples,
        }
    }

    pub fn signature(&self) -> &str {
        self.signature
    }

    pub fn documentation(&self) -> &str {
        self.documentation
    }

    pub fn examples(&self) -> &[&str] {
        self.examples
    }
}

pub fn get_method_docs(func: &str) -> Option<MethodDoc> {
    match func {
        "all" => Some(MethodDoc::new(
            "`all(x)`",
            "Returns `true` if all items in the array `x` is true.",
            &[
            "[true, false, false, true].all() -> false",
            "[true, true, true, true].all() -> true",
        ])),
        "any" => Some(MethodDoc::new(
            "`any(x)`",
            "Returns `true` if any items in the array `x` is true.",
            &[
            "[true, false, false, true].any() -> true",
            "[false, false, false, false].any() -> false",
        ])),
        "atan2" => Some(MethodDoc::new(
            "`atan2(x, y)`",
            "Returns the four quadrant inverse tangent of `x`/`y` in radians between -pi and pi.",
            &[
            "atan2(3, 2) -> 0.982793723247329",
        ])),
        "case" => Some(MethodDoc::new(
            "`case(x, c1, r1, c2, r2, ..., (default))`",
            "Compare `x` to each of `c1`, `c2`, etc. and return the matching `r1`, `r2` of the first match. If no entry matches, a final optional expression can be returned as default.",
            &[
            "case(\"b\", \"a\", 1, \"b\", 2, \"c\", 3, 0) -> 2",
            "case(\"d\", \"a\", 1, \"b\", 2, \"c\", 3, 0) -> 0",
        ])),
        "ceil" => Some(MethodDoc::new(
            "`ceil(x)`",
            "Returns `x` rounded up to the nearest integer.",
            &[
            "ceil(16.2) -> 17",
        ])),
        "chars" => Some(MethodDoc::new(
            "`chars(x)`",
            "Creates an array of characters from a string.",
            &[
            "\"test\".chars() -> [\"t\", \"e\", \"s\", \"t\"]",
        ])),
        "chunk" => Some(MethodDoc::new(
            "`chunk(x, s)`",
            "Converts the list `x` into several lists of length at most `s`",
            &[
            "chunk([1, 2, 3, 4, 5, 6, 7], 3) -> [[1, 2, 3], [4, 5, 6], [7]]",
        ])),
        "coalesce" => Some(MethodDoc::new(
            "`coalesce(a, b, ...)`",
            "Return the first non-null value in the list of values.",
            &[
            "coalesce(null, \"a\", \"b\") -> \"a\"",
        ])),
        "concat" => Some(MethodDoc::new(
            "`concat(x, y, ...)`",
            "Concatenate any number of strings.",
            &[
            "concat(\"Hello, \", \"world!\") -> \"Hello, world!\"",
            "{
    \"externalId\": concat(\"some-prefix:\", input.tag)
}",
        ])),
        "contains" => Some(MethodDoc::new(
            "`contains(x, a)`",
            "Returns `true` if the array or string `x` contains item `a`.",
            &[
            "[1, 2, 3, 4].contains(4) -> true",
            "\"hello world\".contains(\"llo wo\") -> true",
        ])),
        "digest" => Some(MethodDoc::new(
            "`digest(a, b, ...)`",
            "Compute the SHA256 hash of the list of values.",
            &[
            "digest(\"foo\", \"bar\", 123, [1, 2, 3]) -> lDN5G9Qz3fKZM6joQq+1OdF8P1rs2WYrgawlFXflqss=",
        ])),
        "distinct_by" => Some(MethodDoc::new(
            "`distinct_by(x, (a(, b)) => ...)`",
            "Returns a list or object where the elements are distinct by the returned value of the given lambda function. The lambda function either takes list values, or object (value, key) pairs.",
            &[
            "[1, 2, 3, 4, 5].distinct_by(x => x % 2) -> [1, 2]",
        ])),
        "ends_with" => Some(MethodDoc::new(
            "`ends_with(item, substring)`",
            "Returns `true` if `item` ends with `substring`.",
            &[
            "\"hello world\".ends_with(\"world\") -> true",
        ])),
        "except" => Some(MethodDoc::new(
            "`except(x, (v(, k)) => ...)` or `except(x, l)`",
            "Returns a list or object where keys or entries maching the predicate have been removed.
If the second argument is a lambda, it will be given the entry and if it returns `true`, the entry is removed.
If the second argument is a list, any entry also found in this list will be removed.",
            &[
            "{
    \"x-axis\": 13.6,
    \"y-axis\": 63.1,
    \"z-axis\": 1.4,
    \"offset\": 4.3,
    \"power\": \"on\"
}.except([\"offset\", \"power\"])
->
{
    \"x-axis\": 13.6,
    \"y-axis\": 63.1,
    \"z-axis\": 1.4
}",
            "{
    \"a\": 1,
    \"b\": 2,
    \"c\": 3,
    \"d\": 4
}.except((v, k) => v > 2)
->
{
    \"a\": 1,
    \"b\": 2
}",
        ])),
        "filter" => Some(MethodDoc::new(
            "`filter(x, it => ...)`",
            "Removes any item from the list `x` where the lambda function returns `false` or `null`.",
            &[
            "[1, 2, 3, 4].filter(item => item > 2) -> [3, 4]",
            "input.data.map(row => {
    \"timestamp\": to_unix_timestamp(row.StartTime, \"%Y-%m-%dT%H:%M:%S\"),
    \"value\": try_float(row.Value, null),
    \"externalId\": concat(\"prefix/\", column.Name),
    \"type\": \"datapoint\",
}).filter(dp => dp.value is \"number\")",
        ])),
        "flatmap" => Some(MethodDoc::new(
            "`flatmap(x, it => ...)`",
            "Applies the lambda function to every item in the list `x` and flattens the result.

For example, if the lambda function returns a list, the result of the `flatmap` will just be a list instead of a list of lists.",
            &[
            "[[1, 2, 3], [2, 3, 4], [3, 4, 5]].flatmap(list => list.map(item => item + 1))
->
[2, 3, 4, 3, 4, 5, 4, 5, 6]",
            "input.sensorData.flatmap(timeseries =>
    timeseries.values.map(datapoint => {
        \"value\": datapoint.value,
        \"timestamp\": to_unix_timestamp(datapoint.datetime, \"%Y-%m-%dT%H:%M:%S\"),
        \"externalId\": concat(timeseries.location, \"/\", timeseries.sensor),
        \"type\": \"datapoint\"
    })
)",
        ])),
        "float" => Some(MethodDoc::new(
            "`float(x)`",
            "Converts `x` into a floating point number if possible. If the conversion fails, the whole mapping will fail.

Consider using [try_float](#try_float) instead if you need error handling.",
            &[
            "float(\"6.1\") -> 6.1",
        ])),
        "floor" => Some(MethodDoc::new(
            "`floor(x)`",
            "Returns `x` rounded down to the nearest integer.",
            &[
            "floor(16.2) -> 16",
        ])),
        "format_timestamp" => Some(MethodDoc::new(
            "`format_timestamp(x, f)`",
            "Converts the Unix timestamp `x` into a string representation based on the format `f`.

The format is given using the table found [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).",
            &[
            "format_timestamp(1694159249120, \"%Y-%m-%d %H:%M:%S\") -> \"2023-09-08 07:47:29\"",
            "format_timestamp(now(), \"%d/%m - %Y\") -> \"08/09 - 2023\"",
        ])),
        "if" => Some(MethodDoc::new(
            "`if(x, y, (z))`",
            "Returns `y` if `x` evaluates to `true`, otherwise return `z`, or `null` if `z` is omitted.",
            &[
            "if(condition, \"yes\", \"no\")",
            "if(true, \"on\", \"off\") -> \"on\"",
        ])),
        "if_value" => Some(MethodDoc::new(
            "`if_value(item, item => ...)`",
            "Maps a value using a lambda if the value is not null. This is useful if you need to combine parts of some complex object or result of a longer calculation.",
            &[
            "\"hello\".if_value(a => concat(a, \" world\")) -> \"hello world\"",
            "null.if_value(a => a + 1) -> null",
            "[1, 2, 3].if_value(a => a[0] + a[1] + a[2]) -> 6",
        ])),
        "int" => Some(MethodDoc::new(
            "`int(x)`",
            "Converts `x` into an integer if possible. If the conversion fails, the whole mapping will fail.

Consider using [try_int](#try_int) instead if you need error handling.",
            &[
            "int(\"6\") -> 6",
        ])),
        "join" => Some(MethodDoc::new(
            "`join(a, b, ...)`",
            "Returns the union of the given objects or arrays. If a key is present in multiple objects, each instance of the key is overwritten by later objects. Arrays are simply merged.",
            &[
            "join({\"key1\": \"value1\"}, {\"key2\": \"value2\"})
->
{
    \"key1\": \"value1\",
    \"key2\": \"value2\"
}",
            "join([1, 2, 3], [4, 5], [6, 7, 8])
->
[1, 2, 3, 4, 5, 6, 7, 8]",
        ])),
        "length" => Some(MethodDoc::new(
            "`length(x)`",
            "Returns the length on the list, string or object `x`.",
            &[
            "length(\"Hello, world\") -> 12",
            "length([1, 2, 3]) -> 3",
            "length(input.items)",
        ])),
        "log" => Some(MethodDoc::new(
            "`log(x, y)`",
            "Returns the base `y` logarithm of `x`.",
            &[
            "log(16, 2) -> 4.0",
        ])),
        "map" => Some(MethodDoc::new(
            "`map(x, (it(, index)) => ...)`",
            "Applies the lambda function to every item in the list `x`. The lambda takes an optional second input which is the index of the item in the list.

If applied to an object, the first input is the value, and the second is the key. The result is the new value.

If the value is `null`, the lambda is ignored and `map` returns `null`.",
            &[
            "[1, 2, 3, 4].map(number => number * 2) -> [2, 4, 6, 8]",
            "input.data.map(item => {
    \"type\": \"datapoint\",
    \"value\": item.value,
    \"externalId\": concat(\"prefix:\", item.tag),
    \"timestamp\": now()
})",
            "[\"a\", \"b\", \"c\"].map((item, index) => index)
->
[1, 2, 3]",
            "{\"a\": 1, \"b\": 2, \"c\": 3}.map((value, key) => concat(value, key))
->
{\"a\": \"1a\", \"b\": \"2b\", \"c\": \"3c\"}",
        ])),
        "max" => Some(MethodDoc::new(
            "`max(a, b)`",
            "Returns the larger of the two numbers `a` and `b`.",
            &[
            "max(1, 2) -> 2",
        ])),
        "min" => Some(MethodDoc::new(
            "`min(a, b)`",
            "Returns the smaller of the two numbers `a` and `b`.",
            &[
            "min(1, 2) -> 1",
        ])),
        "now" => Some(MethodDoc::new(
            "`now()`",
            "Returns the current time as a millisecond Unix timestamp, that is, the number of milliseconds since midnight 1/1/1970 UTC.",
            &[
            "{
    \"timestamp\": now()
}",
        ])),
        "pairs" => Some(MethodDoc::new(
            "`pairs(x)`",
            "Convert the object `x` into a list of key/value pairs.",
            &[
            "{
    \"a\": 1,
    \"b\": 2,
    \"c\": 3
}.pairs()
->
[{
    \"key\": \"a\",
    \"value\": 1
}, {
    \"key\": \"b\",
    \"value\": 2
}, {
    \"key\": \"c\",
    \"value\": 3
}]",
            "{
    \"x-axis\": 12.4,
    \"y-axis\": 17.3,
    \"z-axis\": 2.1
}.pairs().map(kv => {
    \"timestamp\": now(),
    \"value\": kv.value,
    \"externalId\": kv.key,
    \"type\": \"datapoint\"
})",
        ])),
        "pow" => Some(MethodDoc::new(
            "`pow(x, y)`",
            "Returns `x` to the power of `y`",
            &[
            "pow(5, 3) -> 125.0",
        ])),
        "reduce" => Some(MethodDoc::new(
            "`reduce(x, (acc, val) => ..., init)`",
            "Returns the value obtained by reducing the list `x`. The lambda function is called once for each element in the list `val`, and the returned value is passed as `acc` in the next iteration. The `init` will be given as the initial `acc` for the first call to the lambda function.",
            &[
            "[1, 2, 3, 4, 5].reduce((acc, val) => acc + val, 0) -> 15",
            "[1, 2, 3, 4, 5].reduce((acc, val) => acc * val, 1) -> 120",
        ])),
        "regex_all_captures" => Some(MethodDoc::new(
            "`regex_all_captures(haystack, regex)`",
            "Return an array of objects containing all capture groups from each match of the regex in the haystack. Unnamed capture groups are named after their index, so the match itself is always included as capture group `0`. If no match is found, this returns an empty array.
See [regex_is_match](#regex_is_match) for details on regex support.",
            &[
            "regex_all_captures(\"f123 f45 ff\", \"f(?<v>[0-9]+)\") -> [{ \"0\": \"f123\", \"v\": \"123\" }, { \"0\": \"f45\", \"v\": \"45\" }]",
        ])),
        "regex_all_matches" => Some(MethodDoc::new(
            "`regex_all_matches(haystack, regex)`",
            "Return an array of all the substrings that match the regex. If no match is found, this returns an empty array. Prefer [regex_first_match](#regex_first_match) if all you need is the first match.
See [regex_is_match](#regex_is_match) for details on regex support.",
            &[
            "regex_all_matches(\"tests\", \"t[a-z]\") -> [\"te\", \"ts\"]",
            "regex_all_matches(\"foo bar baz\", \"\\w{3}\") -> [\"foo\", \"bar\", \"baz\"]",
            "regex_all_matches(\"test\", \"not test\") -> []",
        ])),
        "regex_first_captures" => Some(MethodDoc::new(
            "`regex_first_captures(haystack, regex)`",
            "Return an object containing all capture groups from the first match of the regex in the haystack. Unnamed capture groups are named after their index, so the match itself is always included as capture group `0`. If no match is found, this returns null.
See [regex_is_match](#regex_is_match) for details on regex support.",
            &[
            "regex_first_captures(\"test foo bar\", \"test (?<v1>\\w{3}) (\\w{3})\") -> { \"0\": \"test foo bar\", \"v1\": \"foo\", \"2\": \"bar\" }",
        ])),
        "regex_first_match" => Some(MethodDoc::new(
            "`regex_first_match(haystack, regex)`",
            "Return the first substring in the haystack that matches the regex. If no match is found, this returns `null`. Prefer [regex_is_match](#regex_is_match) if all you need is to check for the existence of a match.
See [regex_is_match](#regex_is_match) for details on regex support.",
            &[
            "regex_first_match(\"test\", \"te\") -> \"te\"",
            "regex_first_match(\"te[st]{2}\") -> \"test\"",
        ])),
        "regex_is_match" => Some(MethodDoc::new(
            "`regex_is_match(haystack, regex)`",
            "Return `true` if the haystack matches the regex. Prefer this over the other regex methods if you only need to check for the presence of a match.
Note that we support a limited form of regex without certain complex features like backreferences and look-around. See [here](https://docs.rs/regex/1.11.0/regex/index.html#syntax) for a detailed overview of all the available regex syntax. We recommend using [regex101](https://regex101.com/) with the mode set to `rust` for debugging regex.",
            &[
            "regex_is_match(\"test\", \"te\") -> true",
            "regex_is_match(\"test\", \"^not test$\") -> false",
        ])),
        "regex_replace" => Some(MethodDoc::new(
            "`regex_replace(haystack, regex, replace)`",
            "Replace the first occurence of the regex in the haystack. The replace object supports referencing capture groups using either the index (`$1`) or the name (`$group`). Use `$$` if you need a literal `$` symbol. `${group}` is equivalent to `$group` but lets you specify the group name exactly.
See [regex_is_match](#regex_is_match) for details on regex support.",
            &[
            "regex_replace(\"test\", \"te(?<v>[st]{2})\", \"fa$v\") -> \"fast\"",
        ])),
        "regex_replace_all" => Some(MethodDoc::new(
            "`regex_replace_all(haystack, regex, replace)`",
            "Replace each occurence of the regex in the haystack. See [regex_replace](#regex_replace) for details.",
            &[
            "regex_replace_all(\"tests\", \"t(?<v>[se])\", \"${v}t\") -> etsst",
        ])),
        "replace" => Some(MethodDoc::new(
            "`replace(a, b, c)`",
            "Replaces a string with another string",
            &[
            "\"tomato\".replace(\"tomato\",\"potato\") -> \"potato\"",
            "replace(\"potato\",\"o\",\"a\") -> \"patata\"",
        ])),
        "round" => Some(MethodDoc::new(
            "`round(x)`",
            "Returns `x` rounded to the nearest integer.",
            &[
            "round(16.2) -> 16",
        ])),
        "select" => Some(MethodDoc::new(
            "`select(x, (v(, k)) => ...)` or `select(x, [1, 2, 3])`",
            "Returs a list or object where the lambda returns true. If the second argument is a list, the list values or object keys found in that list are used to select from the source.",
            &[
            "{
    \"x-axis\": 13.6,
    \"y-axis\": 63.1,
    \"z-axis\": 1.4,
    \"offset\": 4.3,
    \"power\": \"on\"
}.select([\"x-axis\", \"y-axis\", \"z-axis\"])
->
{
    \"x-axis\": 13.6,
    \"y-axis\": 63.1,
    \"z-axis\": 1.4
}",
            "{
    \"a\": 1,
    \"b\": 2,
    \"c\": 3
}.select((v, k) => v > 2)
->
{
    \"c\": 3
}",
        ])),
        "slice" => Some(MethodDoc::new(
            "`slice(x, start(, end))`",
            "Creates a sub-array from an array `x` from `start` to `end`. If `end is not specified, go from `start` the end of the array. If `start` or `end` are negative, count from the end of the array.",
            &[
            "[1, 2, 3, 4].slice(1, 3) -> [2, 3]",
            "[1, 2, 3, 4].slice(0, -3) -> [1]",
        ])),
        "split" => Some(MethodDoc::new(
            "`split(a, b)`",
            "Splits string `a` on any occurences of `b`. If `b` is an empty string, this will split on each character, including before the first and after the last.",
            &[
            "\"hello world\".split(\" \") -> [\"hello\", \"world\"]",
            "\"hello\".split(\"\") -> [\"\", \"h\", \"e\", \"l\", \"l\", \"o\", \"\"]",
        ])),
        "starts_with" => Some(MethodDoc::new(
            "`starts_with(item, substring)`",
            "Returns `true` if `item` starts with `substring`.",
            &[
            "\"hello world\".starts_with(\"hello\") -> true",
        ])),
        "string" => Some(MethodDoc::new(
            "`string(x)`",
            "Converts `x` into a string.

`null`s will be converted into empty strings.",
            &[
            "string(true) -> \"true\"",
        ])),
        "string_join" => Some(MethodDoc::new(
            "`string_join(x(, a))`",
            "Returns a string with all the elements of `x`, separated by `a`. If `a` is omitted, the strings will be joined without any separator.",
            &[
            "[\"hello\", \"there\"].string_join(\" \") -> \"hello there\"",
            "[1, 2, 3].string_join() -> \"123\"",
        ])),
        "substring" => Some(MethodDoc::new(
            "`substring(x, start(, end))`",
            "Creates a substring of an input string `x` from `start` to `end`. If `end` is not specified, go from `start` to end of string. If `start` or `end` are negative, count from the end of the string.",
            &[
            "\"hello world\".substring(3, 8) -> \"lo wo\"",
            "\"hello world\".substring(0, -3) -> \"hello wo\"",
        ])),
        "sum" => Some(MethodDoc::new(
            "`sum(x)`",
            "Sums the numbers in the array `x`.",
            &[
            "[1, 2, 3, 4].sum() -> 10",
        ])),
        "tail" => Some(MethodDoc::new(
            "`tail(x(, n))`",
            "Takes the last element of the list `x`. If `n` is given, takes the last `n` elements, and returns a list if `n` > 1.",
            &[
            "[1, 2, 3, 4, 5].tail() -> 5",
            "[1, 2, 3, 4, 5].tail(2) -> [4, 5]",
        ])),
        "to_object" => Some(MethodDoc::new(
            "`to_object(x, val => ...(, val => ...))`",
            "Converts the array `x` into an object by producing the key and value from two lambdas.

The first lambda produces the key, and the second (optional) produces the value. If the second is
left out, the input is used as a value directly.",
            &[
            "[1, 2, 3].to_object(v => string(v + 1)) -> { \"2\": 1, \"3\": 2, \"4\": 3 }",
            "[1, 2, 3].to_object(v => string(v + 1), v => v - 1) -> { \"2\": 0, \"3\": 1, \"4\": 2 }",
            "{\"a\": 1, \"b\": 2, \"c\": 3}.pairs().to_object(pair => pair.key, pair => pair.value) -> {\"a\": 1, \"b\": 2, \"c\": 3}",
        ])),
        "to_unix_timestamp" => Some(MethodDoc::new(
            "`to_unix_timestamp(x, f)`",
            "Converts the string `x` into a millisecond Unix timestamp using the format string `f`.

The format is given using the table found [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).",
            &[
            "to_unix_timestamp(\"2023-05-01 12:43:23\", \"%Y-%m-%d %H:%M:%S\") -> 1682945003000",
            "{
    \"timestamp\": to_unix_timestamp(input.time, \"%Y-%m-%d %H:%M:%S\")
}",
        ])),
        "trim_whitespace" => Some(MethodDoc::new(
            "`trim_whitespace(x)`",
            "Removes any whitespace from the start and end of `x`",
            &[
            "\"  hello   \".trim_whitespace() -> \"hello\"",
        ])),
        "try_bool" => Some(MethodDoc::new(
            "`try_bool(a, b)`",
            "Try convert `a` to a boolean, if it fails, return `b`.",
            &[
            "try_bool(\"true\", null) -> true",
            "try_bool(\"foo\", null) -> null",
        ])),
        "try_float" => Some(MethodDoc::new(
            "`try_float(a, b)`",
            "Try convert `a` to a float, if it fails, return `b`.",
            &[
            "try_float(\"6.2\", 1.2) -> 6.2",
            "try_float(\"4,5\", null) -> 4.5",
        ])),
        "try_int" => Some(MethodDoc::new(
            "`try_int(a, b)`",
            "Try convert `a` to a int, if it fails, return `b`.",
            &[
            "try_int(\"6\", 1) -> 6",
            "try_int(\"4\", null) -> 4",
        ])),
        "zip" => Some(MethodDoc::new(
            "`zip(x, y, ..., (i1, i2, ...) => ...)`",
            "Takes a number of arrays, call the given lambda function on each entry, and return a single array from the result of each call. The returned array will be as long as the longest argument, null will be given for the shorter input arrays when they run out.",
            &[
            "zip([1, 2, 3], [\"a\", \"b\", \"c\"], (a, b) => concat(a, b)) -> [\"1a\", \"2b\", \"3c\"]",
        ])),        _ => None,    }}