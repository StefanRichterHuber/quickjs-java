package io.github.stefanrichterhuber.quickjs.internal;

import java.util.Map.Entry;

import io.github.stefanrichterhuber.quickjs.QuickJSObject;

/**
 * Entry implementation for QuickJSObject. Keys are copied from the JS object
 * when the entry is created. Values are copied from the JS object when the
 * entry is accessed.
 */
public class QuickJSObjectEntry<K, V> implements Entry<K, V> {
    private final QuickJSObject<K, V> parent;
    private final K key;

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

    public int hashCode() {
        final V value = this.getValue();
        return (this.key == null ? 0 : this.key.hashCode()) ^ (value == null ? 0 : value.hashCode());
    }

    public String toString() {
        return this.key + "=" + this.getValue();
    }
}