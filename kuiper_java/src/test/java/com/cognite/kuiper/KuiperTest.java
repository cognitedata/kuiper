package com.cognite.kuiper;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;

import org.junit.Test;

public class KuiperTest {
    public KuiperTest() {}

    @Test
    public void testOk() throws KuiperException {
        var expr = new KuiperExpression("1 + 1");
        assertEquals("2", expr.run());
    }

    @Test
    public void testCompileError() throws KuiperException {
        KuiperException ex = assertThrows(KuiperException.class, () -> new KuiperExpression("1 + floor(5, 5)"));
        assertEquals("Compilation failed: Incorrect number of function args: function floor takes 1 arguments at 4..15", ex.getMessage());
    }

    @Test
    public void testMultipleInputs() throws KuiperException {
        var expr = new KuiperExpression("in1 + in2 + in3", "in1", "in2", "in3");
        assertEquals("6", expr.run("1", "2", "3"));
    }

    @Test
    public void testRunError() throws KuiperException {
        var expr = new KuiperExpression("1 / input", "input");
        KuiperException ex = assertThrows(KuiperException.class, () -> expr.run("0"));
        assertEquals("Divide by zero at 2..3", ex.getMessage());
    }
}
