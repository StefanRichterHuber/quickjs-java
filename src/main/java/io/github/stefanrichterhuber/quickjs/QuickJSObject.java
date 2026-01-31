package io.github.stefanrichterhuber.quickjs;

import java.util.AbstractMap;
import java.util.Collection;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

import io.github.stefanrichterhuber.quickjs.internal.QuickJSObjectEntry;

public class QuickJSObject<K, V> extends AbstractMap<K, V> {
    private static final Logger LOGGER = LogManager.getLogger();

    /**
     * Native pointer to js object
     */
    long ptr;

    /**
     * QuickJSContext this object is bound to.
     */
    private final QuickJSContext ctx;

    /**
     * Clean up native references to this object, must be called eventually to
     * prevent memory leaks
     * 
     * @param ptr Native pointer to the js object
     */
    private static native void closeObject(long ptr);

    /**
     * Returns the size of the object
     * 
     * @param ptr Native pointer to the js object
     * @return size of the object
     */
    private static native int getObjectSize(long ptr, QuickJSContext ctx);

    /**
     * Creates a new native object
     * 
     * @param ctx QuickJSContext to bind the object to
     * @return native pointer to the new object
     */
    private static native long createNativeObject(QuickJSContext ctx);

    /**
     * Sets the value at the given index
     * 
     * @param ptr   Native pointer to the js array
     * @param ctx   QuickJSContext this array is bound to
     * @param key   key to set the value at
     * @param value value to set
     * @return true if the value was set, false otherwise
     */
    private static native boolean setValue(long ptr, QuickJSContext ctx, Object key, Object value);

    /**
     * Returns the value at the given index
     * 
     * @param ptr Native pointer to the js array
     * @param ctx QuickJSContext this array is bound to
     * @param key key to get the value from
     * @return value at the given index
     */
    private static native Object getValue(long ptr, QuickJSContext ctx, Object key);

    /**
     * Returns true if the object contains the given key
     * 
     * @param ptr Native pointer to the js array
     * @param ctx QuickJSContext this array is bound to
     * @param key key to check for
     * @return true if the object contains the given key, false otherwise
     */
    private static native boolean containsKey(long ptr, QuickJSContext ctx, Object key);

    /**
     * Removes the value at the given index
     * 
     * @param ptr Native pointer to the js array
     * @param ctx QuickJSContext this array is bound to
     * @param key key to remove the value from
     * @return true if the value was removed, false otherwise
     */
    private static native boolean removeValue(long ptr, QuickJSContext ctx, Object key);

    /**
     * Returns the keys of the object
     * 
     * @param ptr Native pointer to the js array
     * @param ctx QuickJSContext this array is bound to
     * @return set of keys
     */
    private static native Set<?> keySet(long ptr, QuickJSContext ctx);

    /**
     * Creates a new QuickJSArray from a native array pointer. This should only be
     * called from a native context!
     * 
     * @param arrayPtr Native pointer to the js array
     * @param ctx      QuickJSContext this array is bound to
     */
    private QuickJSObject(long arrayPtr, final QuickJSContext ctx) {
        if (ctx == null) {
            throw new NullPointerException("Context must not be null");
        }
        this.ctx = ctx;
        this.ptr = arrayPtr;
        // This array is closed, when the underlying context is closed
        ctx.addDependentResource(this::close);
    }

    /**
     * Creates a new QuickJSObject
     * 
     * @param ctx QuickJSContext this object is bound to
     */
    public QuickJSObject(final QuickJSContext ctx) {
        if (ctx == null) {
            throw new NullPointerException("Context must not be null");
        }
        this.ctx = ctx;
        this.ptr = createNativeObject(ctx);
        // This array is closed, when the underlying context is closed
        ctx.addDependentResource(this::close);
    }

    /**
     * Creates a new QuickJSObject from a Map
     * 
     * @param ctx QuickJSContext this object is bound to
     * @param src Map to copy values from
     */
    public QuickJSObject(final QuickJSContext ctx, final Map<K, V> src) {
        this(ctx);
        this.putAll(src);
    }

    /**
     * Returns the native pointer to the native object. First check if this
     * object is still active at all (a native QuickJS object exists)
     * 
     * @return native pointer to an active QuickJS object.
     */
    long getContextPointer() {
        if (ptr == 0) {
            throw new IllegalStateException("Object is closed");
        }
        return this.ptr;
    }

    private void close() throws Exception {
        if (this.ptr != 0) {
            closeObject(ptr);
            LOGGER.debug("Closed JSObject with id {}", ptr);
            ptr = 0;
        }
    }

    @Override
    public boolean containsKey(Object key) {
        return containsKey(getContextPointer(), ctx, key);
    }

    @Override
    public boolean containsValue(Object value) {
        return values().contains(value);
    }

    /**
     * Unlike the standard Map.entrySet(), this method returns a snapshot of the
     * entry set at the time of the call. Changes to the underlying QuickJS object
     * after the call will only reflected on the values of the entry set, not on
     * the keys.
     */
    @Override
    public Set<Entry<K, V>> entrySet() {
        return keySet().stream().map(k -> new QuickJSObjectEntry<>(this, k)).collect(Collectors.toSet());
    }

    @Override
    public V get(Object key) {
        if (key == null) {
            throw new NullPointerException("Key must not be null");
        }
        return (V) getValue(ptr, ctx, key);
    }

    @Override
    public boolean isEmpty() {
        return this.size() == 0;
    }

    @Override
    public Set<K> keySet() {
        return (Set<K>) keySet(getContextPointer(), ctx);
    }

    @Override
    public V put(K key, V value) {
        if (key == null) {
            throw new NullPointerException("Key must not be null");
        }
        setValue(ptr, ctx, key, value);
        return value;
    }

    @Override
    public V remove(Object key) {
        V value = get(key);
        removeValue(getContextPointer(), ctx, key);
        return value;
    }

    @Override
    public int size() {
        return getObjectSize(ptr, ctx);
    }

    /**
     * Returns the values of the object. Unlike the standard Map.values() this is a
     * copy of the values at the time of the call and not a view on the object.
     * 
     * @return collection of values
     */
    @Override
    public Collection<V> values() {
        return entrySet().stream().map(Entry::getValue).collect(Collectors.toList());
    }

}
