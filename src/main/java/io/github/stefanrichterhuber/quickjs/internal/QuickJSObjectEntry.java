package io.github.stefanrichterhuber.quickjs.internal;

import java.util.Map.Entry;
import java.util.Objects;

import io.github.stefanrichterhuber.quickjs.QuickJSObject;

/**
 * Entry implementation for QuickJSObject. Keys are copied from the JS object
 * when the entry is created. Values are copied from the JS object when the
 * entry is accessed.
 * 
 * @param <K> the type of keys maintained by this map. Must be a String, Number
 *            or Boolean.
 * @param <V> the type of mapped values. Any type that can be converted to a
 *            QuickJS value (including other maps)
 */
public class QuickJSObjectEntry<K, V> implements Entry<K, V> {
    private final QuickJSObject<K, V> parent;
    private final K key;

    /**
     * Creates a QuickJSObjectEntry for the given QuickJSObject and key.
     * 
     * @param parent parent QuickJSObject
     * @param key    key of the entry
     */
    public QuickJSObjectEntry(QuickJSObject<K, V> parent, K key) {
        this.parent = parent;
        this.key = key;
    }

    @Override
    public K getKey() {
        return key;
    }

    @Override
    public V getValue() {
        return parent.get(key);
    }

    @Override
    public V setValue(V value) {
        final V oldValue = getValue();
        parent.put(key, value);
        return oldValue;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) {
            return true;
        }
        if (o == null || !(o instanceof Entry)) {
            return false;
        }
        final Entry<?, ?> that = (Entry<?, ?>) o;
        return Objects.equals(getKey(), that.getKey()) && Objects.equals(getValue(), that.getValue());
    }

    @Override
    public int hashCode() {
        final V value = this.getValue();
        return (this.key == null ? 0 : this.key.hashCode()) ^ (value == null ? 0 : value.hashCode());
    }

    @Override
    public String toString() {
        return this.key + "=" + this.getValue();
    }
}