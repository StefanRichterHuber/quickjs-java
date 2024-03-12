package com.github.stefanrichterhuber.quickjs;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.Collection;
import java.util.HashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.function.BiFunction;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Supplier;

/**
 * QuickJSContext is a independent namespace with its own set of globals. With
 * this instance we interact with the QuickJS runtime, set and get global
 * variables and finally evaluate JS code. It is not thread safe!
 */
public class QuickJSContext implements AutoCloseable {
    /**
     * Invocation handler for java dynamic proxies which passes all method
     * invocations to the underlying scripting context
     */
    private static class Proxy implements InvocationHandler {
        private final String namespace;
        private final QuickJSContext context;

        Proxy(QuickJSContext context, String namespace) {
            this.namespace = namespace;
            this.context = context;
        }

        @Override
        public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
            if (method.isDefault()) {
                return InvocationHandler.invokeDefault(proxy, method, args);
            }
            String name = method.getName();
            if (namespace != null) {
                name = namespace + "." + name;
            }
            return context.invoke(name, args);
        }

        @SuppressWarnings("unchecked")
        static <T> T create(
                QuickJSContext context, String namespace,
                Class<T> clazz) {
            return (T) java.lang.reflect.Proxy.newProxyInstance(clazz.getClassLoader(), new Class[] { clazz },
                    new Proxy(context, namespace));
        }

    }

    private final QuickJSRuntime runtime;
    private long ptr;

    private native long createContext(long runtimePtr);

    private native void closeContext(long ptr);

    private native void setGlobal(long ptr, String name, Object value);

    private native Object getGlobal(long ptr, String name);

    private native Object eval(long ptr, String script);

    private native Object invoke(long ptr, String name, Object... args);

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

    /**
     * Creates a new QuickJSContext instance from a QuickJSRuntime
     * 
     * @param runtime
     */
    QuickJSContext(QuickJSRuntime runtime) {
        this.runtime = runtime;
        this.ptr = createContext(runtime.getRuntimePointer());
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
        this.checkForDependentResources(result);
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
     * Adds a global variable to the context. Array values are copied value by
     * value to an JS array.
     * 
     * @param name  Name of the variable
     * @param value Value of the variable
     */
    public <T> void setGlobal(String name, T[] value) {
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
        this.runtime.scriptStarted();
        try {
            final Object result = this.eval(getContextPointer(), script);
            checkForDependentResources(result);
            return result;
        } finally {
            this.runtime.scriptFinished();
        }
    }

    /**
     * Invokes a JavaScript function and returns the result. It could be both a Java
     * function passed to the context as well as a previously defined native JS
     * function in the context.
     * 
     * @param name Name of the function to invoke
     * @param args Arguments to pass to the function
     * @return Result of the call
     */
    public Object invoke(String name, Object... args) {
        this.runtime.scriptStarted();
        try {
            final Object result = this.invoke(getContextPointer(), name, args);
            checkForDependentResources(result);
            return result;
        } finally {
            this.runtime.scriptFinished();
        }
    }

    /**
     * Creates a script-backed dynamic proxy for the given interface class. All
     * , but default methods (!), from the interface are passed as invoke to the
     * script
     * context.
     * 
     * @param <T>       Type of the interface to proxy
     * @param namespace Optional name space (all method calls are prefixed with it.
     *                  namespace = 'obj', method name = 'f' -> obj.f() is called);
     *                  can be null
     * @param clazz     Class of the interface to be proxied
     * @return Proxied instance of the interface
     */
    public <T> T getInterface(String namespace, Class<T> clazz) {
        if (clazz.isInterface()) {
            return Proxy.create(this, namespace, clazz);
        } else {
            throw new IllegalArgumentException("Class '" + clazz + "' is not an interface!");
        }
    }

    // Checks for context dependent resources like QuickJSFunction and add them to
    // the clean up list
    void checkForDependentResources(Object result) {
        if (result instanceof QuickJSFunction) {
            var f = (QuickJSFunction) result;
            dependedResources.add(f);
            f.setCtx(this);
        }
        if (result instanceof Collection) {
            for (Object o : (Collection<?>) result) {
                checkForDependentResources(o);
            }
        }
        if (result instanceof Map) {
            checkForDependentResources(((Map<?, ?>) result).values());
        }
    }

    @Override
    public boolean equals(Object obj) {
        return obj instanceof QuickJSContext && ((QuickJSContext) obj).ptr == this.ptr;
    }

    @Override
    public int hashCode() {
        return ptr == 0 ? 0 : Objects.hash(ptr);
    }
}
