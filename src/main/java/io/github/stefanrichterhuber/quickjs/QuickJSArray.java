package io.github.stefanrichterhuber.quickjs;

import java.util.AbstractList;
import java.util.Collection;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/**
 * Wrapper for native JS arrays. Exposed in the form of a Java List.
 * 
 * @param <T> type of the elements in the array. Any type that can be converted
 *            to a QuickJS value (including other lists)
 */
public final class QuickJSArray<T> extends AbstractList<T> {
    private static final Logger LOGGER = LogManager.getLogger();

    /**
     * Native pointer to js array
     */
    long ptr;

    /**
     * QuickJSContext this array is bound to.
     */
    private final QuickJSContext ctx;

    /**
     * Clean up native references to this array, must be called eventually to
     * prevent memory leaks
     * 
     * @param ptr Native pointer to the js array
     */
    private static native void closeArray(long ptr);

    /**
     * Returns the size of the array
     * 
     * @param ptr Native pointer to the js array
     * @return size of the array
     */
    private static native int getArraySize(long ptr, QuickJSContext ctx);

    /**
     * Creates a new native array
     * 
     * @param ctx QuickJSContext to bind the array to
     * @return native pointer to the new array
     */
    private static native long createNativeArray(QuickJSContext ctx);

    /**
     * Sets the value at the given index
     * 
     * @param ptr   Native pointer to the js array
     * @param ctx   QuickJSContext this array is bound to
     * @param index index to set the value at
     * @param value value to set
     * @return true if the value was set, false otherwise
     */
    private static native boolean setValue(long ptr, QuickJSContext ctx, int index, Object value);

    /**
     * Sets the value at the given index
     * 
     * @param ptr   Native pointer to the js array
     * @param ctx   QuickJSContext this array is bound to
     * @param index index to set the value at
     * @param value value to set
     * @return true if the value was set, false otherwise
     */
    private static native boolean addValue(long ptr, QuickJSContext ctx, int index, Object value);

    /**
     * Returns the value at the given index
     * 
     * @param ptr   Native pointer to the js array
     * @param ctx   QuickJSContext this array is bound to
     * @param index index to get the value from
     * @return value at the given index
     */
    private static native Object getValue(long ptr, QuickJSContext ctx, int index);

    /**
     * Removes the value at the given index
     * 
     * @param ptr   Native pointer to the js array
     * @param ctx   QuickJSContext this array is bound to
     * @param index index to remove the value from
     * @return true if the value was removed, false otherwise
     */
    private static native boolean removeValue(long ptr, QuickJSContext ctx, int index);

    /**
     * Creates a new QuickJSArray from a native array pointer. This should only be
     * called from a native context!
     * 
     * @param arrayPtr Native pointer to the js array
     * @param ctx      QuickJSContext this array is bound to
     */
    private QuickJSArray(long arrayPtr, final QuickJSContext ctx) {
        if (ctx == null) {
            throw new NullPointerException("Context must not be null");
        }
        this.ctx = ctx;
        this.ptr = arrayPtr;
        // This array is closed, when the underlying context is closed
        ctx.addDependentResource(this::close);
    }

    /**
     * Creates a new empty QuickJSArray
     * 
     * @param ctx QuickJSContext to bind the array to
     */
    public QuickJSArray(final QuickJSContext ctx) {
        if (ctx == null) {
            throw new NullPointerException("Context must not be null");
        }
        this.ctx = ctx;
        this.ptr = createNativeArray(ctx);
        // This array is closed, when the underlying context is closed
        ctx.addDependentResource(this::close);
    }

    /**
     * Creates a new QuickJSArray from a collection of values. The order of the
     * elements in the array will be the same as the order of the elements in the
     * collection.
     * 
     * @param ctx QuickJSContext to bind the array to
     * @param src Collection of values to add to the array
     */
    public QuickJSArray(final QuickJSContext ctx, final Collection<T> src) {
        this(ctx);
        this.addAll(src);
    }

    /**
     * Returns the native pointer to the native array. First check if this
     * array is still active at all (a native QuickJS array exists)
     * 
     * @return native pointer to an active QuickJS array.
     */
    long getContextPointer() {
        if (ptr == 0) {
            throw new IllegalStateException("Array is closed");
        }
        return this.ptr;
    }

    @Override
    public T get(int index) {
        if (index < 0 || index >= size()) {
            throw new IndexOutOfBoundsException("Invalid index: " + index);
        }
        return (T) getValue(this.getContextPointer(), this.ctx, index);
    }

    @Override
    public int size() {
        return getArraySize(this.getContextPointer(), this.ctx);
    }

    @Override
    public boolean add(T value) {
        LOGGER.info("Adding value to QuickJSArray: {}", value);
        final int len = getArraySize(this.getContextPointer(), this.ctx);
        return setValue(this.getContextPointer(), this.ctx, len, value);
    }

    @Override
    public void add(int index, T value) {
        if (index < 0 || index > size()) {
            throw new IndexOutOfBoundsException("Invalid index: " + index);
        }
        addValue(this.getContextPointer(), this.ctx, index, value);
    }

    @Override
    public T set(int index, T value) {
        if (index < 0 || index >= size()) {
            throw new IndexOutOfBoundsException("Invalid index: " + index);
        }

        final T oldValue = get(index);
        setValue(this.getContextPointer(), this.ctx, index, value);

        return oldValue;
    }

    @Override
    public T remove(int index) {
        if (index < 0 || index >= size()) {
            throw new IndexOutOfBoundsException("Invalid index: " + index);
        }
        T oldValue = this.get(index);
        removeValue(this.getContextPointer(), this.ctx, index);
        return oldValue;
    }

    /**
     * Closes the native QuickJS array. Makes the array invalid and this
     * list instance unusable.
     * 
     * @throws Exception
     */
    private void close() throws Exception {
        if (this.ptr != 0) {
            closeArray(ptr);
            LOGGER.debug("Closed JSArray with id {}", ptr);
            ptr = 0;
        }
    }

}
