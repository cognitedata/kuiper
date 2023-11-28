---
pagination_next: null
pagination_prev: null
title: Built-in functions
---

# All built-in functions

## atan2

`atan2(x, y)`

Returns the inverse tangent of `x`/`y` in radians between -pi and pi.

**Code example**
```
atan2(3, 2) -> 0.982793723247329
```

## case

`case(x, c1, r1, c2, r2, ..., (default))`

Compare `x` to each of `c1`, `c2`, etc. and return the matching `r1`, `r2` of the first match. If no entry matches, a final optional expression can be returned as default.

**Code examples**
```
case("b", "a", 1, "b", 2, "c", 3, 0) -> 2
```
```
case("d", "a", 1, "b", 2, "c", 3, 0) -> 0
```

## ceil

`ceil(x)`

Returns `x` rounded up to the nearest integer.

**Code example**
```
ceil(16.2) -> 17
```

## chars

`chars(x)`

Creates an array of characters from a string.

**Code example**
```
"test".chars() -> ["t", "e", "s", "t"]
```

## chunk

`chunk(x, s)`

Converts the list `x` into several lists of length at most `s`

**Code example**
```
chunk([1, 2, 3, 4, 5, 6, 7], 3) -> [[1, 2, 3], [4, 5, 6], [7]]
```

## concat

`concat(x, y, ...)`

Concatenate any number of strings.

**Code examples**
```
concat("Hello, ", "world!") -> "Hello, world!"
```
```
{
    "externalId": concat("some-prefix:", input.tag)
}
```

## distinct_by

`distinct_by(x, s)`

Returns a list or object where the elements are distinct by the returned value of the given lambda function. The lambda function either takes list values, or object (value, key) pairs

**Code example**
```
[1, 2, 3, 4, 5].distinct_by(x => x % 2) -> [1, 2]
```

## except

`except(x, (v(, k)) => ...)` or `except(x, l)`

Returns a list or object where the elements are distinct by the returned value of the given lambda function. The lambda function either takes list values, or object (value, key) pairs

**Code examples**
```
{
    "x-axis": 13.6,
    "y-axis": 63.1,
    "z-axis": 1.4,
    "offset": 4.3,
    "power": "on"
}.except(["offset", "power"])
->
{
    "x-axis": 13.6,
    "y-axis": 63.1,
    "z-axis": 1.4
}
```
```
{
    "a": 1,
    "b": 2,
    "c": 3,
    "d": 4
}.except((v, k) => v > 2)
->
{
    "a": 1,
    "b": 2
}
```

## filter

`filter(x, it => ...)`

Removes any item from the list `x` where the lambda function returns a falsy value.

**Code examples**
```
[1, 2, 3, 4].filter(item => item > 2) -> [3, 4]
```
```
input.data.map(row => {
    "timestamp": to_unix_timestamp(row.StartTime, "%Y-%m-%dT%H:%M:%S"),
    "value": try_float(row.Value, null),
    "externalId": concat("prefix/", column.Name),
    "type": "datapoint",
}).filter(dp => dp.value is "number")
```

## flatmap

`flatmap(x, it => ...)`

Applies the lambda function to every item in the list `x` and flattens the result.

For example, if the lambda function returns a list, the result of the `flatmap` will just be a list instead of a list of lists.

**Code examples**
```
[[1, 2, 3], [2, 3, 4], [3, 4, 5]].flatmap(list => list.map(item => item + 1))
->
[2, 3, 4, 3, 4, 5, 4, 5, 6]
```
```
input.sensorData.flatmap(timeseries =>
    timeseries.values.map(datapoint => {
        "value": datapoint.value,
        "timestamp": to_unix_timestamp(datapoint.datetime, "%Y-%m-%dT%H:%M:%S"),
        "externalId": concat(timeseries.location, "/", timeseries.sensor),
        "type": "datapoint"
    })
)
```

## float

`float(x)`

Converts `x` into a floating point number if possible. If the conversion fails, the whole mapping will fail.

Consider using [try_float](#try_float) instead if you need error handling.

**Code example**
```
float("6.1") -> 6.1
```

## floor

`floor(x)`

Returns `x` rounded down to the nearest integer.

**Code example**
```
floor(16.2) -> 16
```

## format_timestamp

`format_timestamp(x)`

Converts the Unix timestamp `x` into a string representation based on the format `f`.

The format is given using the table found [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

**Code examples**
```
format_timestamp(1694159249120, "%Y-%m-%d %H:%M:%S") -> "2023-09-08 07:47:29"
```
```
format_timestamp(now(), "%d/%m - %Y") -> "08/09 - 2023"
```

## if

`if(x, y, (z))`

Returns `y` if `x` evaluates to `true`, otherwise return `z`, or `null` if `z` is omitted.

**Code examples**
```
if(condition, "yes", "no")
```
```
if(true, "on", "off") -> "on"
```

## int

`int(x)`

Converts `x` into an integer if possible. If the conversion fails, the whole mapping will fail.

Consider using [try_int](#try_int) instead if you need error handling.

**Code example**
```
int("6") -> 6
```

## join

`join(a, b)`

Returns the union of the two objects `a` and `b`. If a key is present in both objects, `b` takes precedent.

**Code example**
```
join({"key1": "value1"}, {"key2": "value2"})
->
{
    "key1": "value1",
    "key2": "value2"
}
```

## length

`length(x)`

Returns the length on the list, string or object `x`.

**Code examples**
```
length("Hello, world") -> 12
```
```
length([1, 2, 3]) -> 3
```
```
length(input.items)
```

## log

`log(x, y)`

Returns the base `y` logarithm of `x`.

**Code example**
```
log(16, 2) -> 4.0
```

## map

`map(x, it => ...)`

Applies the lambda function to every item in the list `x`.

**Code examples**
```
[1, 2, 3, 4].map(number => number * 2) -> [2, 4, 6, 8]
```
```
input.data.map(item => {
    "type": "datapoint",
    "value": item.value,
    "externalId": concat("prefix:", item.tag),
    "timestamp": now()
})
```

## now

`now()`

Returns the current time as a millisecond Unix timestamp, ie number of milliseconds since midnight 1/1/1970 UTC.

**Code example**
```
{
    "timestamp": now()
}
```

## pairs

`pairs(x)`

Convert the object `x` into a list of key/value pairs.

**Code examples**
```
{
    "a": 1,
    "b": 2,
    "c": 3
}.pairs()
->
[{
    "key": "a",
    "value": 1
}, {
    "key": "b",
    "value": 2
}, {
    "key": "c",
    "value": 3
}]
```
```
{
    "x-axis": 12.4,
    "y-axis": 17.3,
    "z-axis": 2.1
}.pairs().map(kv => {
    "timestamp": now(),
    "value": kv.value,
    "externalId": kv.key,
    "type": "datapoint"
})
```

## pow

`pow(x, y)`

Returns `x` to the power of `y`

**Code example**
```
pow(5, 3) -> 125.0
```

## reduce

`reduce(x, (acc, val) => ..., init)`

Returns the value obtained by reducing the list `x`. The lambda function is called once for each element in the list `val`, and the returned value is passed as `acc` in the next iteration. The `init` will be given as the initial `acc` for the first call to the lambda function.

**Code examples**
```
[1, 2, 3, 4, 5].reduce((acc, val) => acc + val, 0) -> 15
```
```
[1, 2, 3, 4, 5].reduce((acc, val) => acc * val, 1) -> 120
```

## round

`round(x)`

Returns `x` rounded to the nearest integer.

**Code example**
```
round(16.2) -> 16
```

## select

`select(x, (v(, k)) => ...)` or `select(x, [1, 2, 3])`

Returs a list or object where the lambda returns true. If the second argument is a list, the list values or object keys found in that list are used to select from the source.

**Code examples**
```
{
    "x-axis": 13.6,
    "y-axis": 63.1,
    "z-axis": 1.4,
    "offset": 4.3,
    "power": "on"
}.select(["x-axis", "y-axis", "z-axis"])
->
{
    "x-axis": 13.6,
    "y-axis": 63.1,
    "z-axis": 1.4
}
```
```
{
    "a": 1,
    "b": 2,
    "c": 3
}.select((v, k) => v > 2)
->
{
    "c": 3
}
```

## slice

`slice(x, start(, end))`

Creates a sub-array from an array `x` from `start` to `end`. If `end is not specified, go from `start` the end of the array. If `start` or `end` are negative, count from the end of the array.

**Code examples**
```
[1, 2, 3, 4].slice(1, 3) -> [2, 3]
```
```
[1, 2, 3, 4].slice(0, -3) -> [1]
```

## split

`split(a, b)`

Splits string `a` on any occurences of `b`. If `b` is an empty string, this will split on each character, including before the first and after the last.

**Code examples**
```
"hello world".split(" ") -> ["hello", "world"]
```
```
"hello".split("") -> ["", "h", "e", "l", "l", "o", ""]
```

## string

`string(x)`

Converts `x` into a string.

`null`s will be converted into empty strings.

**Code example**
```
string(true) -> "true"
```

## substring

`substring(x, start(, end))`

Creates a substring of an input string `x` from `start` to `end`. If `end` is not specified, go from `start` to end of string. If `start` or `end` are negative, count from the end of the string.

**Code examples**
```
"hello world".substring(3, 8) -> "lo wo"
```
```
"hello world".substring(0, -3) -> "hello wo"
```

## tail

`tail(x(, n))`

Takes the last element of the list `x`. If `n` is given, takes the last `n` elements, and returns a list if `n` > 1.

**Code examples**
```
[1, 2, 3, 4, 5].tail() -> 5
```
```
[1, 2, 3, 4, 5].tail(2) -> [4, 5]
```

## to_unix_timestamp

`to_unix_timestamp(x, f)`

Converts the string `x` into a millisecond unix timestamp using the format string `f`.

The format is given using the table found [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

**Code examples**
```
to_unix_timestamp("2023-05-01 12:43:23", "%Y-%m-%d %H:%M:%S") -> 1682945003000
```
```
{
    "timestamp": to_unix_timestamp(input.time, "%Y-%m-%d %H:%M:%S")
}
```

## trim_whitespace

`trim_whitespace(x)`

Removes any whitespace from the start and end of `x`

**Code example**
```
"  hello   ".trim_whitespace() -> "hello"
```

## try_bool

`try_bool(a, b)`

Try convert `a` to a boolean, if it fails, return `b`

**Code examples**
```
try_bool("true", null) -> true
```
```
try_bool("foo", null) -> null
```

## try_float

`try_float(a, b)`

Try convert `a` to a float, if it fails, return `b`

**Code examples**
```
try_float("6.2", 1.2) -> 6.2
```
```
try_float("4,5", null) -> 4.5
```

## try_int

`try_int(a, b)`

Try convert `a` to a int, if it fails, return `b`

**Code examples**
```
try_int("6", 1) -> 6
```
```
try_int("4", null) -> 4
```

## zip

`zip(x, y, ..., (i1, i2, ...) => ...)`

Takes a number of arrays, call the given lambda function on each entry, and return a single array from the result of each call. The returned array will be as long as the longest argument, null will be given for the shorter input arrays when they run out.

**Code example**
```
zip([1, 2, 3], ['a', 'b', 'c'], (a, b) => concat(a, b)) -> ["1a", "2b", "3c"]
```
