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

This project uses [maturin](https://www.maturin.rs/) to manage dependencies and
such. To set up your local development environment, first make sure maturin is
installed:

``` commanline
pip install -U maturin
```

Then, you can enter or exit a virtual environment with the `kuiper` package
installed by sourcing the `enter.sh` or `exit.sh` scripts:

``` commandline
source enter.sh
```

``` commandline
source exit.sh
```

Whenever you change the code, you need to rebuild. Instead of using `cargo`
directly, use `maturin`:

``` commandline
maturin develop
```

This will build and install the python package into the current environment,
which is the virtual environment created by the `enter.sh` script.
