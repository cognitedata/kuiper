package com.cognite.kuiper;

public class KuiperException extends Exception {
    public long start;
    public long end;

    public KuiperException(String message, long start, long end) {
        super(message);
        this.start = start;
        this.end = end;
    }

    public KuiperException(String message) {
        super(message);
    }
}
