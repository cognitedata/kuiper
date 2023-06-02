# Python bindings for Kuiper

Python bindings for Kuiper so you can compile and run transformations from a
Python runtime.

``` python
import kuiper

expression = kuiper.compile_expression("input.value + 15", ["input"])
result = expression.run("{ \"value\": 27 }")

print(result)
```


## Development

We use [PyO3](https://pyo3.rs) to create the bindings.

This project uses [maturin](https://www.maturin.rs/) to manage dependencies and
such, instead of poetry. To set up your local development environment, first
make sure maturin is installed:

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

