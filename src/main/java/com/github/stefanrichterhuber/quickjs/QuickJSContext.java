package com.github.stefanrichterhuber.quickjs;

import java.io.Closeable;
import java.io.IOException;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.function.Function;

public class QuickJSContext implements Closeable {
    private long ptr;

    private native long createContext(long runtimePtr);

    private native void closeContext(long ptr);

    private native void setGlobal(long ptr, String name, Object value);

    private native void setGlobal(long ptr, String name, Function<Object, Object> f);

    private native Object eval(long ptr, String script);

    @Override
    public void close() throws IOException {
        if (ptr != 0) {
            closeContext(ptr);
            ptr = 0;
        }
    }

    QuickJSContext(QuickJSRuntime runtime) {
        ptr = createContext(runtime.getRuntimePointer());
    }

    /**
     * Returns the native pointer to the QuickJS context
     * 
     * @return native pointer
     */
    long getContextPointer() {
        if (ptr == 0) {
            throw new IllegalStateException("QuickJSContext is closed");
        }
        return this.ptr;
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, int value) {
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, BigInteger value) {
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, String value) {
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, double value) {
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, boolean value) {
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, BigDecimal value) {
        // TODO rquickjs currently does not support BigDecimal directly
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global function to the context.
     * 
     * @param name  Name of the function
     * @param value Value of the function
     */
    public void setGlobal(String name, Function<Object, Object> f) {
        this.setGlobal(getContextPointer(), name, f);
    }

    /**
     * Evaluates a JavaScript script and returns the result.
     * 
     * @param script Script to execute
     * @return Result from the script
     */
    public Object eval(String script) {
        return this.eval(getContextPointer(), script);
    }
}
