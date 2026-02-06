package com.cognite.kuiper;

import java.lang.ref.Cleaner;

public class KuiperExpression {
    private long expression;

    static Cleaner cleaner = Cleaner.create();

    public KuiperExpression(String input, String... known_inputs) throws KuiperException {
        this.expression = Kuiper.compile_expression(input, known_inputs);
        long ptr = this.expression;
        cleaner.register(this, () -> Kuiper.free_expression(ptr));
    }

    public String run(String... input) throws KuiperException {
        return Kuiper.run_expression(this.expression, input);
    }
}
