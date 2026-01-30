package com.github.stefanrichterhuber.quickjs;

import java.util.Collection;
import java.util.HashMap;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.stream.Collectors;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

public class QuickJSObject<K, V> implements AutoCloseable, Map<K, V> {
    private static final Logger LOGGER = LogManager.getLogger();

    private class QuickJSObjectEntry implements Entry<K, V> {
        private final K key;

        public QuickJSObjectEntry(K key) {
            this.key = key;
        }

        @Override
        public K getKey() {
            return key;
        }

        @Override
        public V getValue() {
            return QuickJSObject.this.get(key);
        }

        @Override
        public V setValue(V value) {
            final V oldValue = getValue();
            QuickJSObject.this.put(key, value);
            return oldValue;
        }

        public int hashCode() {
            final V value = this.getValue();
            return (this.key == null ? 0 : this.key.hashCode()) ^ (value == null ? 0 : value.hashCode());
        }

        public String toString() {
            return this.key + "=" + this.getValue();
        }
    }

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

    public QuickJSObject(final QuickJSContext ctx) {
        if (ctx == null) {
            throw new NullPointerException("Context must not be null");
        }
        this.ctx = ctx;
        this.ptr = createNativeObject(ctx);
        // This array is closed, when the underlying context is closed
        ctx.addDependentResource(this::close);
    }

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

    @Override
    public void close() throws Exception {
        if (this.ptr != 0) {
            closeObject(ptr);
            LOGGER.debug("Closed JSObject with id {}", ptr);
            ptr = 0;
        }
    }

    @Override
    public void clear() {
        keySet().forEach(this::remove);
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
        return keySet().stream().map(QuickJSObjectEntry::new).collect(Collectors.toSet());
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
    public void putAll(Map<? extends K, ? extends V> m) {
        if (m == null) {
            throw new NullPointerException("Map must not be null");
        }
        for (Entry<? extends K, ? extends V> entry : m.entrySet()) {
            this.put(entry.getKey(), entry.getValue());
        }
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
        new HashMap<>().hashCode();
        return entrySet().stream().map(Entry::getValue).collect(Collectors.toList());
    }

    @Override
    public int hashCode() {
        int hashCode = 1;

        for (Entry<K, V> e : entrySet()) {
            hashCode = 31 * hashCode + (e == null ? 0 : e.hashCode());
        }

        return hashCode;
    }

    @Override
    public boolean equals(Object o) {
        // Stolen from java.util.ArrayList
        if (o == this) {
            return true;
        } else if (!(o instanceof Map)) {
            return false;
        } else {
            new HashMap<>();
            Map other = (Map) o;

            if (other.size() != this.size()) {
                return false;
            }

            for (Entry<K, V> entry : entrySet()) {
                final K key = entry.getKey();
                final V value = entry.getValue();
                if (!other.containsKey(key) || !Objects.equals(other.get(key), value)) {
                    return false;
                }
            }
            return true;
        }
    }

    @Override
    public String toString() {
        return entrySet().toString();
    }

}
