package com.github.stefanrichterhuber.quickjs;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.Collection;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.function.BiFunction;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Supplier;

public class QuickJSContext implements AutoCloseable {
    private long ptr;

    private native long createContext(long runtimePtr);

    private native void closeContext(long ptr);

    private native void setGlobal(long ptr, String name, Object value);

    private native Object getGlobal(long ptr, String name);

    private native Object eval(long ptr, String script);

    /**
     * Keep a reference to all functions received to prevent memory leaks which
     * results in errors when closing the context
     */
    private final Set<AutoCloseable> dependedResources = new HashSet<>();

    @Override
    public void close() throws Exception {
        if (ptr != 0) {
            for (AutoCloseable f : dependedResources) {
                f.close();
            }
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
     * Returns the value of the global variable with the given name.
     * 
     * @param name Name of the variable
     */
    public Object getGlobal(String name) {
        Object result = this.getGlobal(getContextPointer(), name);
        this.checkForDependendResources(result);
        return result;
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, Map<String, Object> value) {
        this.setGlobal(getContextPointer(), name, value);
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
    private void setGlobal(String name, BigInteger value) {
        // FIXME currently not supported by rquickjs
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    private void setGlobal(String name, long value) {
        // FIXME currently not supported by rquickjs
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
     * Adds a global variable to the context. Iterable values are copied value by
     * value to an JS array.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public void setGlobal(String name, Iterable<?> value) {
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global variable to the context.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    private void setGlobal(String name, BigDecimal value) {
        // TODO rquickjs currently does not support BigDecimal directly
        this.setGlobal(getContextPointer(), name, value);
    }

    /**
     * Adds a global function to the context.
     * 
     * @param name  Name of the function
     * @param value Value of the function
     */
    public void setGlobal(String name, Function<?, ?> f) {
        this.setGlobal(getContextPointer(), name, f);
    }

    /**
     * Adds a global function to the context.
     * 
     * @param name  Name of the function
     * @param value Value of the function
     */
    public void setGlobal(String name, QuickJSFunction f) {
        this.setGlobal(getContextPointer(), name, f);
    }

    /**
     * Adds a global function to the context.
     * 
     * @param name  Name of the function
     * @param value Value of the function
     */
    public void setGlobal(String name, Supplier<?> f) {
        this.setGlobal(getContextPointer(), name, f);
    }

    /**
     * Adds a global function to the context.
     * 
     * @param name  Name of the function
     * @param value Value of the function
     */
    public void setGlobal(String name, BiFunction<?, ?, ?> f) {
        this.setGlobal(getContextPointer(), name, f);
    }

    /**
     * Adds a global function to the context.
     * 
     * @param name  Name of the function
     * @param value Value of the function
     */
    public void setGlobal(String name, Consumer<?> f) {
        this.setGlobal(getContextPointer(), name, f);
    }

    /**
     * Evaluates a JavaScript script and returns the result.
     * 
     * @param script Script to execute
     * @return Result from the script
     */
    public Object eval(String script) {
        final Object result = this.eval(getContextPointer(), script);
        checkForDependendResources(result);
        return result;
    }

    // Checks for context dependend resources like QuickJSFunction and add them to
    // the clean up list
    void checkForDependendResources(Object result) {
        if (result instanceof QuickJSFunction) {
            var f = (QuickJSFunction) result;
            dependedResources.add(f);
            f.setCtx(this);
        }
        if (result instanceof Collection) {
            for (Object o : (Collection<?>) result) {
                checkForDependendResources(o);
            }
        }
        if (result instanceof Map) {
            checkForDependendResources(((Map<?, ?>) result).values());
        }
    }
}
