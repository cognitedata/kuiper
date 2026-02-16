# Kuiper CLI

This pacakage contains the Kuiper CLI. The CLI will operate on an input JSON file, and either an inline expression (with the `-e` argument) or an expression file (with the `-f` argument):

``` commandline
$ cat input.json
[1, 2, 3, 4]
$ kuiper -e "input.sum()" input.json
10
```

You can also use Kuiper with pipes:

``` commandline
$ cat input.json | kuiper -e "input.sum()"
10
```

Run `kuiper --help` for a full list of possible arguments.

The CLI also contains a REPL, which you can launch by just running `kuiper`.

To install the Kuiper CLI, either
 * Download pre-built binaries from the [GitHub releases page](https://github.com/cognitedata/kuiper/releases)
 * Fetch the last version from crates.io:
   ``` commandline
   cargo install kuiper_cli
   ```
 * Build and install from source. Clone the git repository, and run
   ``` commandline
   cargo install --path kuiper_cli
   ```
   from the root directory.

The command `kuiper` should now be available.
