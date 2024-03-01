package com.github.stefanrichterhuber.quickjs;

import java.util.Objects;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

public class QuickJSFunction implements AutoCloseable {
    private static final Logger LOGGER = LogManager.getLogger();

    long ptr;

    private QuickJSContext ctx;

    private native void closeFunction(long ptr);

    private native Object callFunction(long ptr, Object... args);

    // TODO add name of the function from js?
    public QuickJSFunction(long ptr) {
        this.ptr = ptr;
    }

    @Override
    public void close() throws RuntimeException {
        if (this.ptr != 0) {
            LOGGER.debug("Close QuickJSFunction " + ptr);
            closeFunction(ptr);
            ptr = 0;
        }
    }

    public Object call(Object... args) {
        if (ptr != 0) {
            final Object result = this.callFunction(ptr, args);
            if (this.ctx != null) {
                ctx.checkForDependendResources(result);
            } else {
                LOGGER.warn("QuickJSFunction not bound to QuickJSContext - might result in memory leaks");
            }

            return result;
        } else {
            throw new IllegalStateException("QuickJSFunction already destroyed!");
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
