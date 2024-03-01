package com.github.stefanrichterhuber.quickjs;

public class QuickJSFunction implements AutoCloseable {
    private long ptr;

    private native void closeFunction(long ptr);

    private native Object callFunction(long ptr, Object... args);

    public QuickJSFunction(long ptr) {
        this.ptr = ptr;
    }

    @Override
    public void close() throws Exception {
        if (this.ptr != 0) {

        }
        ptr = 0;
    }

    public Object call(Object... args) {
        if (ptr != 0) {
            return this.callFunction(ptr, args);
        } else {
            throw new IllegalStateException("QuickJSFunction already destroyed!");
        }
    }
}
