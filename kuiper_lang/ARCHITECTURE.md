# Kuiper architecture

Kuiper is built using a fairly normal compiler architecture, where compilation proceeds in layers. The final result is a large enum that forms an in-memory tree, `ExpressionType`, where expressions will call each other recursively. A kuiper expression is thus always composed of other kuiper expressions, except in base cases like the `Constant` or `Selector` expression types, which provide constants and external input respsectively.

For example, the expression `input.test.value + 5` has the following in-memory structure:

 - Operator (+)
   - Selector (0, "test", "value")
   - Constant (5)

The label `input` is known to the compiler, and transformed into an index in the input array during compilation. In this case `0`. Operator precedence is also expressed this way, for example `1 * 2 + 3` is the expression tree

 - Operator (+)
   - Operator (*)
     - Constant (1)
     - Constant (2)
   - Constant (3)

Since expressions are executed recursively, deeper expressions are executed first, which preserves operator precedence in this case. The naive approach consumes left-to-right, and would result in an expression where `+` is executed first.

If you look at `compile_expression_with_config` in `src/compiler/mod.rs` you'll see fairly clearly how the compiler is divided into stages, each performing a separate task.

## Lexer

The lexer parses the expression into tokens. This greedily consumes the input stream, but ignores whitespace and comments. So the expression

`concat(string(5 * 4), "hello" /* comment */)`

is transformed into the stream

`[concat, (, string, (, 5, *, 4, ), ",", "hello", ), Comment]`

Note that we don't capture the content of comments, but we do preserve the original position in the input string. Each symbol becomes its own token. You can find the token definition in `src/lexer/token.rs`. We use a tokenizer called "Logos". Before we pass this to the next stage we do a few minor modifications:

 - We transform `)` followed by `=>` into a single special token `)=>`. This is a workaround for an ambiguity due to the fact we use an LR-1 parser.
 - We remove comments.

## Parser

The parser takes the modified stream of tokens and produces an Abstract Syntax Tree (AST). The AST itself is defined in `src/parse/ast.rs`, while the parser is written in a special parser language called "lalrpop". It's an LR-1 parser, meaning it traverses the tokens from left to right with a 1 token lookahead.

If you need to modify the language itself you'll need to modify the syntax file, `kuiper.lalrpop`. This is a kind of declarative language, where you give names to sequences of tokens, which is used to automatically generate code for a parser. If the syntax is invalid you'll get a compile error.

For example, the entry point, `Program`, is defined as the list of macros defined by an expression. An `Expr` is defined as an `Op2Expr`, which is defined as either an `Op3Expr` or `Op2Expr Op2 Op3Expr`, where `Op2` is just `||`, the _lowest_ priority operator. This is how we get operator precedence, and there's a recursive definition here that goes all the way to `Op8Expr`.

The next step is `Term`, which is some sub-expression. Either a constant, a variable, an array, an object, a function call, an `if` expression, a selector, etc. This also contains parenthesized expressions, giving them the highest priority. Note that this wraps all the way back to `Expr`. This kind of recursive definition is very common in parsers.

## Execution tree

The AST is used to build the "execution tree". This is the first stage where we use the input arguments to the function, like the list of known inputs. `src/compiler/exec_tree` contains the `ExecTreeBuilder` which recursively traverses the AST, maintaining an internal state containing information such as macros and the list of known inputs.

This process also replaces the input strings (like `input` above), with their indexes in the input array.

## Optimization

We do a single, simple, optimization pass. Kuiper is almost entirely deterministic, so what we do is:

 - Check if the expression is deterministic (i.e. no sub-expression contains non-deterministic expressions such as `now()`)
 - If it is, attempt to execute the expression.
 - If the result is `SourceMissingError` or the expression is non-deterministic, go one step down and attempt to execute each child expression in the same way.
 - If we get a result, replace the expression with a constant.
 - If we get any _other_ error, that's a hard error, because this would fail on every execution of the expression.

This means that the final compiled expression for something like

`5 * 5 + input.foo`

is actually just

`10 + input.foo`

## Type checking

There is a final unfinished type checking layer, though this is disabled by default, as it is missing implementations for most functions. This effectively executes the expression in "type space", meaning we look at whether the expression _can_ succeed. For example, the expression `input.foo / "test"` can _never_ succeed, since dividing by a string is invalid.
