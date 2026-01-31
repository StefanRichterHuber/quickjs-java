package io.github.stefanrichterhuber.quickjs;

import java.util.Objects;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/**
 * A QuickJSFunction represents a callable JavaScript function in a
 * QuickJSContext. Since the QuickJSFunction represents a native resource, it
 * has to be closed if it is no longer needed. This is managed by the underlying
 * QuickJSContext. This class is not meant to be instantiated from the Java
 * runtime, but only from the native library, therefore its constructor is
 * package-private.
 */
public final class QuickJSFunction implements VariadicFunction<Object> {
    private static final Logger LOGGER = LogManager.getLogger();

    /**
     * Java script name of the function
     */
    private final String name;

    /**
     * Native pointer to js function
     */
    long ptr;

    /**
     * QuickJSContext this function is bound to.
     */
    private final QuickJSContext ctx;

    /**
     * Clean up native references to this function, must be called eventually to
     * prevent memory leaks
     * 
     * @param ptr Native pointer to the js function
     */
    private static native void closeFunction(long ptr);

    /**
     * Calls the JS function with the given arguments
     * 
     * @param ptr  Native pointer to the js function
     * @param args Arguments to pass to the function
     * @return Result of the function call
     */
    private native Object callFunction(long ptr, Object... args);

    /**
     * Creates a new QuickJSFunction instance. This constructor is meant to be
     * called by the native library and therefore is not public
     * 
     * @param ptr     Native pointer to the js function. Must not be 0.
     * @param name    JS name of the function (might be null)
     * @param context QuickJSContext this function is bound to. Used for resource
     *                management, must not be null.
     */
    QuickJSFunction(long ptr, String name, QuickJSContext context) {
        if (ptr == 0) {
            throw new IllegalArgumentException("Pointer must not be 0");
        }
        if (context == null) {
            throw new IllegalArgumentException("Context must not be null");
        }
        this.ptr = ptr;
        this.ctx = context;
        this.name = name;
        // This function is closed, when the underlying context is closed
        context.addDependentResource(this::close);
    }

    /**
     * Resource management is delegated to the QuickJSContext of the function.
     * Therefore it is not necessary to give the user the ability to close the
     * underlying native resources
     */
    private void close() throws RuntimeException {
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
    @Override
    public Object apply(Object... args) {
        if (ptr != 0) {
            final Object result = this.callFunction(ptr, args);
            LOGGER.trace("Invoked QuickJSFunction with id {} -> {}", ptr, result);
            return result;
        } else {
            throw new IllegalStateException("QuickJSFunction already closed!");
        }
    }

    /**
     * Javascript name of the function, might be null.
     * 
     * @return Name of the function
     */
    public String getName() {
        return this.name;
    }

    @Override
    public String toString() {
        return "QuickJSFunction " + (this.name == null || this.name.isBlank() ? "<unknown>" : this.name)
                + "() with id " + this.ptr;
    }

    /**
     * QuickJSFunctions are equal by their native pointer.
     */
    @Override
    public boolean equals(Object obj) {
        return obj instanceof QuickJSFunction && ((QuickJSFunction) obj).ptr == this.ptr;
    }

    @Override
    public int hashCode() {
        return ptr == 0 ? 0 : Objects.hash(ptr);
    }
}
