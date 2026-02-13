#include "../kuiper.h"
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

int main() {
    return test_simple_expression();
}
