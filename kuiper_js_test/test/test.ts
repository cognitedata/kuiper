import { compile_expression, CompilerConfig, KuiperError } from '@cognite/kuiper_js';
import { strict as assert } from 'assert';

describe('kuiper_js WASM module', function () {
    it('compiling and running should work', function () {
        const expr = compile_expression("1 + 2", []);
        const res = expr.run([]);
        assert.equal(res, 3);
    });

    it('Compile error should be thrown for invalid expression', function () {
        try {
            compile_expression("1 / 0", []);
            assert.fail("Expected compile error");
        } catch (e) {
            assert.ok(e instanceof KuiperError);
            assert.equal(e.message, "Compilation failed: Divide by zero at 2..3");
            assert.equal(e.start, 2);
            assert.equal(e.end, 3);
        }
    });

    it('Compiling should work with inputs', function () {
        const expr = compile_expression("a + b", ["a", "b"]);
        const res = expr.run(1, 2);
        assert.equal(res, 3);
    });

    it('Inputs can be complex', function () {
        const expr = compile_expression("a.x + b[1]", ["a", "b"]);
        const res = expr.run({ x: 1 }, [1, 2, 3]);
        assert.equal(res, 3);
    });

    it('completions should provide suggestions', function () {
        const expr = compile_expression("a.f", ["a"]);
        const completions = expr.run_get_completions({ foo: 1, bar: 2 });
        assert.deepEqual(completions.get_completions_at(2), ["bar", "foo"]);
    });

    it('custom functions work', function () {
        const config = new CompilerConfig();
        config.add_custom_function("foo", (x: number) => x * 2);
        const expr = compile_expression("foo(a)", ["a"], config);
        const res = expr.run(3);
        assert.equal(res, 6);
    });

    it('exceptions in custom functions are propagated', function () {
        const config = new CompilerConfig();
        config.add_custom_function("foo", (x: number) => { throw new Error("Custom function error"); });
        const expr = compile_expression("foo(a)", ["a"], config);
        try {
            expr.run(3);
            assert.fail("Expected error from custom function");
        } catch (e) {
            assert.ok(e instanceof KuiperError);
            assert.equal(e.message, "Error: Custom function error at 0..6");
        }
    });
});
