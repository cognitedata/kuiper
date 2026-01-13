# Kuiper, the Cognite mapping language

This package contains Java bindings for the Kuiper programming language, letting you build and call Kuiper expressions from Java code.

The language itself is documented [here](https://docs.cognite.com/cdf/integration/guides/extraction/hosted_extractors/kuiper_concepts).

```java
var expr = new KuiperExpression("in1 + in2.test", "in1", "in2");
expr.run("1", "{\"test\": 2}");
```

This package requires `libkuiper_java` somewhere on the library path. You'll find this file in `target/release` if you have built it using `cargo build --release`. To add that path to the library path, set `LD_LIBRARY_PATH="$LD_LIBRARY_PATH:/path/to/kuiper/target/release"`.

## Testing

To test, set `LD_LIBRARY_PATH` as described above, then call `mvn test`.
