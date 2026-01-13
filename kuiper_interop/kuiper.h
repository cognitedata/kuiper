#ifndef KUIPER_H
#define KUIPER_H

// A header file documenting the kuiper interop API.

// An opaque type representing a compiled kuiper expression.
// This is allocated by the `compile_expression` function and should be
// freed by the `destroy_expression` function.
// Internally this is a complex rust type that is not exposed here.
typedef struct ExpressionType ExpressionType;


// An error returned by kuiper functions.
struct KuiperError {
    char* error;
    bool is_error;
    unsigned long start;
    unsigned long end;
};

// The result of compiling a kuiper expression. Either `error` is set, or `result` is set.
struct CompileResult {
    KuiperError error;
    ExpressionType* result;
};

// Compile a kuiper expression with the given input argument names.
CompileResult* compile_expression(const char* expression, const char** inputs, size_t input_count);

// The result of running a kuiper expression. Either `error` is set, or `result` is set.
struct TransformResult {
    KuiperError error;
    char* result;
};

// Run a compiled kuiper expression with the given input data.
TransformResult* run_expression(const char** data, size_t input_count, ExpressionType* expr);

// Free a string allocated by rust.
void destroy_string(char* data);

// Convert an expression to its string representation.
char* expression_to_string(ExpressionType* expr);

// Destroy a transform result, this is called from external code to safely dispose of results
// allocated by `run_expression` after the result has been extracted.
void destroy_transform_result(TransformResult* result);

// Destroy a `CompileResult` and return the `ExpressionType` it contains.
// This does not check whether `result` is null and may return a null pointer.
ExpressionType* get_expression_from_compile_result(CompileResult* result);

// Destroy an expression allocated by `compile_expression`.
void destroy_expression(ExpressionType* expr);

// Destroy a compile result allocated by `compile_expression`.
void destroy_compile_result(CompileResult* result);

#endif
