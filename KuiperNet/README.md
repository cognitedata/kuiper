# Kuiper, the Cognite mapping language

This package contains .NET bindings for the Kuiper programming language, letting you build and call Kuiper expressions from .NET code.

The language itself is documented [here](https://docs.cognite.com/cdf/integration/guides/extraction/hosted_extractors/kuiper_concepts).

Usage

```c#
var expr = new KuiperExpression("in1 + in2.test", ["in1", "in2"]);
expr.Run("1", "{\"test\": 2}")
```
