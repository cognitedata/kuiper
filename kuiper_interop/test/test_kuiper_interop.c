#include "../kuiper.h"
#include <malloc.h>
#include <stdio.h>
#include <string.h>

int test_simple_expression() {
    CompileResult *compile_result = compile_expression("a + b", (const char *[]){"a", "b"}, 2);

    if (compile_result->error.is_error) {
        fprintf(stderr, "Error compiling expression: %s\n", compile_result->error.error);
        destroy_compile_result(compile_result);
        return 1;
    }

    ExpressionType *expr = get_expression_from_compile_result(compile_result);

    TransformResult *transform_result = run_expression((const char *[]){"1", "2"}, 2, expr);

    int error = 0;

    if (transform_result->error.is_error) {
        fprintf(stderr, "Error running expression: %s\n", transform_result->error.error);
        error = 1;
        goto cleanup;
    }

    if (strcmp(transform_result->result, "3") != 0) {
        fprintf(stderr, "Expected result '3', got '%s'\n", transform_result->result);
        error = 1;
        goto cleanup;
    } else {
        printf("Test passed: 'a + b' with a=1 and b=2 gives %s\n", transform_result->result);
    }

cleanup:
    destroy_transform_result(transform_result);
    destroy_expression(expr);
    return error;
}

CustomFunctionResult custom_function(const char **args, size_t arg_count) {
    if (arg_count > 0 && strcmp(args[0], "\"hello\"") == 0) {
        char *result = strdup("\"world\"");
        CustomFunctionResult res = {.is_error = false, .data = result, .free_payload = result, .free_data = free};
        return res;
    } else {
        char *result = strdup("\"unknown\"");
        CustomFunctionResult res = {.is_error = false, .data = result, .free_payload = result, .free_data = free};
        return res;
    }
}

int test_expression_with_custom_function() {
    CompilerConfig *config = new_compiler_config();
    config_add_custom_function(config, "my_func", custom_function);

    CompileResult *compile_result = compile_expression_with_config("my_func('hello')", NULL, 0, config);

    if (compile_result->error.is_error) {
        fprintf(stderr, "Error compiling expression: %s\n", compile_result->error.error);
        destroy_compile_result(compile_result);
        destroy_compiler_config(config);
        return 1;
    }

    ExpressionType *expr = get_expression_from_compile_result(compile_result);

    TransformResult *transform_result = run_expression(NULL, 0, expr);

    int error = 0;

    if (transform_result->error.is_error) {
        fprintf(stderr, "Error running expression: %s\n", transform_result->error.error);
        error = 1;
        goto cleanup;
    }

    if (strcmp(transform_result->result, "\"world\"") != 0) {
        fprintf(stderr, "Expected result '\"world\"', got '%s'\n", transform_result->result);
        error = 1;
        goto cleanup;
    } else {
        printf("Test passed: 'my_func(\"hello\")' gives %s\n", transform_result->result);
    }

cleanup:
    destroy_transform_result(transform_result);
    destroy_expression(expr);
    destroy_compiler_config(config);
    return error;
}

int main() {
    int r = test_simple_expression();
    if (r != 0)
        return r;
    r = test_expression_with_custom_function();
    if (r != 0)
        return r;
    return 0;
}
