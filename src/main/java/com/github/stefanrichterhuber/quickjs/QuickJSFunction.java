package com.github.stefanrichterhuber.quickjs;

import java.util.Objects;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/**
 * A QuickJSFunction represents a callable JavaScript function in a
 * QuickJSContext. To ensure memory safety the function must be cleaned up
 * (closed) after usage.
 */
public class QuickJSFunction {
    private static final Logger LOGGER = LogManager.getLogger();

    /**
     * Native pointer to js function
     */
    long ptr;

    /**
     * QuickJSContext this function is bound to. Might be null
     */
    private QuickJSContext ctx;

    private static native void closeFunction(long ptr);

    private native Object callFunction(long ptr, Object... args);

    // TODO add name of the function from js?
    public QuickJSFunction(long ptr, QuickJSContext context) {
        if (ptr == 0) {
            throw new IllegalArgumentException("Pointer must not be 0");
        }
        if (context == null) {
            throw new IllegalArgumentException("Context must not be null");
        }
        this.ptr = ptr;
        this.ctx = context;
        context.addDependentResource(this::close);
    }

    void close() throws RuntimeException {
        if (this.ptr != 0) {
            closeFunction(ptr);
            LOGGER.debug("Closed QuickJSFunction with id {}", ptr);
            ptr = 0;
        }
    }

    /**
     * Invokes the function with the given arguments. Supports all argument types
     * supported by QuickJSContext in general
     * 
     * @param args Function arguments must match in number and type the arguments
     *             expected by the JS runtime
     * @return Result of the function call
     */
    public Object call(Object... args) {
        if (ptr != 0) {
            final Object result = this.callFunction(ptr, args);
            LOGGER.trace("Invoked QuickJSFunction with id {} -> {}", ptr, result);
            return result;
        } else {
            throw new IllegalStateException("QuickJSFunction already closed!");
        }
    }

    @Override
    public boolean equals(Object obj) {
        return obj instanceof QuickJSFunction && ((QuickJSFunction) obj).ptr == this.ptr;
    }

    @Override
    public int hashCode() {
        return ptr == 0 ? 0 : Objects.hash(ptr);
    }
}
