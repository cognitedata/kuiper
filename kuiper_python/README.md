# Python bindings for Kuiper

Python bindings for Kuiper so you can compile and run transformations from a
Python runtime.

The language itself is documented [here](https://docs.cognite.com/cdf/integration/guides/extraction/hosted_extractors/kuiper_concepts).

``` python
from kuiper import compile_expression

expression = compile_expression('{"theAnswer": input.numericValue + 27}', ["input"])
value = expression.run('{"numericValue": 15}')
print(value)
```

The `compile_expression` function might raise a `KuiperCompileError`, and
otherwise returns a `KuiperExpression` object. The `KuiperExpression.run(...)`
method might raise a `KuiperRuntimeError`. Both of these exceptions are
subclasses of the `KuiperError` base class.

The packakge is available on [PyPI](pypi.org/project/cognite-kuiper/). To use
it in your project, add it to your project file with your project manager of
choice. For example, using `uv`:

``` commandline
uv add cognite-kuiper
```

## Development

We use [PyO3](https://pyo3.rs) to create the bindings.

This project uses [uv](https://docs.astral.sh/uv/) to manage dependencies and
such. To set up your local development environment, first make sure uv is
installed:

``` commandline
pip install -U uv
```

`uv` will automatically recompile the Rust code whenever it changes and install
it in the virual environment.

To run the test suite, run

``` commandline
uv run pytest
```
