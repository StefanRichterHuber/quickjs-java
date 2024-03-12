package com.github.stefanrichterhuber.quickjs;

import java.lang.ref.Cleaner.Cleanable;
import java.util.Objects;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/**
 * A QuickJSFunction represents a callable JavaScript function in a
 * QuickJSContext. To ensure memory safety the function must be cleaned up
 * (closed) after usage.
 */
public class QuickJSFunction implements AutoCloseable {
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

    private final Cleanable cleanJob;

    // TODO add name of the function from js?
    public QuickJSFunction(long ptr) {
        this.ptr = ptr;
        this.cleanJob = QuickJSRuntime.CLEANER.register(this, new CleanJob(ptr));
    }

    private static class CleanJob implements Runnable {
        private long ptr;

        public CleanJob(final long ptr) {
            this.ptr = ptr;
        }

        @Override
        public void run() {
            if (this.ptr != 0) {
                closeFunction(ptr);
                LOGGER.debug("Closed QuickJSFunction with id {}", ptr);
                ptr = 0;
            }
        }
    }

    @Override
    public void close() throws RuntimeException {
        cleanJob.clean();
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
            if (this.ctx != null) {
                ctx.checkForDependentResources(result);
            } else {
                LOGGER.debug("QuickJSFunction with id {} not bound to QuickJSContext - might result in memory leaks",
                        ptr);
            }

            return result;
        } else {
            throw new IllegalStateException("QuickJSFunction already closed!");
        }
    }

    void setCtx(QuickJSContext ctx) {
        this.ctx = ctx;
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
