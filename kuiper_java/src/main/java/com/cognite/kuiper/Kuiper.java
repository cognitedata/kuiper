package com.cognite.kuiper;

class Kuiper {
    public static native long compile_expression(String input, String[] known_inputs) throws KuiperException;

    public static native String run_expression(long expression, String[] inputs) throws KuiperException;

    public static native void free_expresion(long expression);

    static {
        System.loadLibrary("kuiper_java");
    }
}
