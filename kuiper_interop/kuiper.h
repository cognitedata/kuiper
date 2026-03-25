#ifndef KUIPER_H
#define KUIPER_H

#include <stdbool.h>
#include <stddef.h>

// A header file documenting the kuiper interop API.

// An opaque type representing a compiled kuiper expression.
// This is allocated by the `compile_expression` function and should be
// freed by the `destroy_expression` function.
// Internally this is a complex rust type that is not exposed here.
typedef struct ExpressionType ExpressionType;

// An error returned by kuiper functions.
typedef struct KuiperError {
    char *error;
    bool is_error;
    unsigned long start;
    unsigned long end;
} KuiperError;

// The result of compiling a kuiper expression. Either `error` is set, or `result` is set.
typedef struct CompileResult {
    KuiperError error;
    ExpressionType *result;
} CompileResult;

// Compile a kuiper expression with the given input argument names.
CompileResult *compile_expression(const char *expression, const char **inputs, size_t input_count);

// The result of running a kuiper expression. Either `error` is set, or `result` is set.
typedef struct TransformResult {
    KuiperError error;
    char *result;
} TransformResult;

// Run a compiled kuiper expression with the given input data.
TransformResult *run_expression(const char **data, size_t input_count, ExpressionType *expr);

// Free a string allocated by rust.
void destroy_string(char *data);

// Convert an expression to its string representation.
char *expression_to_string(ExpressionType *expr);

// Destroy a transform result, this is called from external code to safely dispose of results
// allocated by `run_expression` after the result has been extracted.
void destroy_transform_result(TransformResult *result);

// Destroy a `CompileResult` and return the `ExpressionType` it contains.
// This does not check whether `result` is null and may return a null pointer.
ExpressionType *get_expression_from_compile_result(CompileResult *result);

// Destroy an expression allocated by `compile_expression`.
void destroy_expression(ExpressionType *expr);

// Destroy a compile result allocated by `compile_expression`.
void destroy_compile_result(CompileResult *result);

// An opaque type representing a compiler configuration.
typedef struct CompilerConfig CompilerConfig;

// Create a new compiler configuration with default settings.
CompilerConfig *new_compiler_config();

// Destroy a compiler configuration allocated by `new_compiler_config`.
void destroy_compiler_config(CompilerConfig *config);

// Set the optimizer operation limit for a compiler configuration.
void config_set_optimizer_operation_limit(CompilerConfig *config, long limit);

// Set the maximum number of macro expansions for a compiler configuration.
void config_set_max_macro_expansions(CompilerConfig *config, int limit);

// The result of a custom function.
// If `is_error` is true, then `data` contains an error message.
// Otherwise, `data` contains the result of the function as a JSON string.
typedef struct CustomFunctionResult {
    // Indicates whether the custom function resulted in an error.
    bool is_error;
    // The result of the custom function as a JSON string, or an error message if `is_error` is true.
    char *data;
    // A pointer to data passed to the `free_data` function for cleanup. This is typically the same as `data`, but can
    // be a different pointer if needed.
    void *free_payload;
    // A function pointer to a function that can be called to free the memory allocated for `data` and any associated
    // resources. The `free_payload` pointer will be passed to this function when it is called.
    void (*free_data)(void *);
} CustomFunctionResult;

// Add a custom function to a compiler configuration. The `implementation` function will be called
// when the custom function is invoked in a kuiper expression. The `implementation` function should
// return a `CustomFunctionResult` containing the result of the function or an error message.
int config_add_custom_function(CompilerConfig *config, const char *name,
                               CustomFunctionResult (*implementation)(const char **args, size_t arg_count));

// Compile a kuiper expression with the given input argument names and a custom compiler configuration.
CompileResult *compile_expression_with_config(const char *expression, const char **inputs, size_t input_count,
                                              CompilerConfig *config);

#endif
