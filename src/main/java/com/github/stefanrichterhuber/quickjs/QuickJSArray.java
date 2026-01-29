package com.github.stefanrichterhuber.quickjs;

import java.util.Collection;
import java.util.Iterator;
import java.util.List;
import java.util.ListIterator;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

public class QuickJSArray<T> implements AutoCloseable, List<T> {
    private static final Logger LOGGER = LogManager.getLogger();

    /**
     * Iterator for this array
     */
    private class QuickJSListIterator implements ListIterator<T> {

        int index;

        private QuickJSListIterator(int startIndex) {
            this.index = startIndex;
        }

        @Override
        public boolean hasNext() {
            return index < size();
        }

        @Override
        public T next() {
            return get(index++);
        }

        @Override
        public boolean hasPrevious() {
            return index > 0;
        }

        @Override
        public T previous() {
            return get(index--);
        }

        @Override
        public int nextIndex() {
            return index;
        }

        @Override
        public int previousIndex() {
            return index - 1;
        }

        @Override
        public void remove() {
            QuickJSArray.this.remove(index);
        }

        @Override
        public void set(T e) {
            QuickJSArray.this.set(index, e);
        }

        @Override
        public void add(T e) {
            QuickJSArray.this.add(index, e);
        }

    }

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
    private static native int getArraySize(long ptr);

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
     * Returns the value at the given index
     * 
     * @param ptr   Native pointer to the js array
     * @param ctx   QuickJSContext this array is bound to
     * @param index index to get the value from
     * @return value at the given index
     */
    private static native Object getValue(long ptr, QuickJSContext ctx, int index);

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

    public QuickJSArray(final QuickJSContext ctx) {
        if (ctx == null) {
            throw new NullPointerException("Context must not be null");
        }
        this.ctx = ctx;
        this.ptr = createNativeArray(ctx);
        // This array is closed, when the underlying context is closed
        ctx.addDependentResource(this::close);
    }

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
    public boolean add(T value) {
        final int len = getArraySize(this.getContextPointer());
        return setValue(this.getContextPointer(), this.ctx, len, value);
    }

    @Override
    public void add(int arg0, T arg1) {
        // TODO we need to shift all elements to the right of the index to the right
        // by one
        // and then insert the new element at the index

        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'add'");
    }

    @Override
    public boolean addAll(Collection<? extends T> c) {
        if (c == null) {
            throw new NullPointerException("Collection must not be null");
        }
        for (T value : c) {
            if (!add(value)) {
                return false;
            }
        }
        return true;
    }

    @Override
    public boolean addAll(int index, Collection<? extends T> c) {
        if (c == null) {
            throw new NullPointerException("Collection must not be null");
        }
        if (index < 0 || index > size()) {
            throw new IndexOutOfBoundsException("Index " + index + " is out of bounds");
        }
        int pos = index;
        for (T value : c) {
            this.add(pos, value);
            pos++;
        }
        return true;
    }

    @Override
    public void clear() {
        for (int i = 0; i < size(); i++) {
            remove(i);
        }
    }

    @Override
    public boolean contains(Object o) {
        for (int i = 0; i < size(); i++) {
            if (get(i).equals(o)) {
                return true;
            }
        }
        return false;
    }

    @Override
    public boolean containsAll(Collection<?> c) {
        if (c == null) {
            throw new NullPointerException("Collection must not be null");
        }
        for (Object o : c) {
            if (!contains(o)) {
                return false;
            }
        }
        return true;
    }

    @SuppressWarnings("unchecked")
    @Override
    public T get(int index) {
        if (index < 0 || index >= size()) {
            throw new IndexOutOfBoundsException("Invalid index: " + index);
        }
        return (T) getValue(this.getContextPointer(), this.ctx, index);
    }

    @Override
    public int indexOf(Object o) {
        for (int i = 0; i < size(); i++) {
            if (get(i).equals(o)) {
                return i;
            }
        }
        return -1;
    }

    @Override
    public boolean isEmpty() {
        return size() == 0;
    }

    @Override
    public Iterator<T> iterator() {
        return listIterator();
    }

    @Override
    public int lastIndexOf(Object o) {
        for (int i = size() - 1; i >= 0; i--) {
            if (get(i).equals(o)) {
                return i;
            }
        }
        return -1;
    }

    @Override
    public ListIterator<T> listIterator() {
        return listIterator(0);
    }

    @Override
    public ListIterator<T> listIterator(int index) {
        if (index < 0 || index > size()) {
            throw new IndexOutOfBoundsException("Invalid index: " + index);
        }
        return new QuickJSListIterator(index);
    }

    @Override
    public boolean remove(Object o) {
        final int index = indexOf(o);
        if (index == -1) {
            return false;
        }
        remove(index);
        return true;
    }

    @Override
    public T remove(int index) {
        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'remove'");
    }

    @Override
    public boolean removeAll(Collection<?> c) {
        if (c == null) {
            throw new NullPointerException("Collection must not be null");
        }
        for (Object o : c) {
            remove(o);
        }
        return true;
    }

    @Override
    public boolean retainAll(Collection<?> c) {
        if (c == null) {
            throw new NullPointerException("Collection must not be null");
        }
        for (Object o : c) {
            if (!contains(o)) {
                remove(o);
            }
        }
        return true;
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
    public int size() {
        return getArraySize(this.getContextPointer());
    }

    @Override
    public List<T> subList(int fromIndex, int toIndex) {
        if (fromIndex < 0 || toIndex > size() || fromIndex > toIndex) {
            throw new IndexOutOfBoundsException("Invalid index range");
        }
        // TODO native copy of elements?

        // TODO Auto-generated method stub
        throw new UnsupportedOperationException("Unimplemented method 'subList'");
    }

    @SuppressWarnings("unchecked")
    @Override
    public Object[] toArray() {
        return toArray((T[]) new Object[size()]);
    }

    @SuppressWarnings("unchecked")
    @Override
    public <T> T[] toArray(T[] array) {
        if (array == null) {
            throw new NullPointerException("Array must not be null");
        }
        if (array.length < size()) {
            array = (T[]) new Object[size()];
        }
        for (int i = 0; i < size(); i++) {
            array[i] = (T) get(i);
        }
        return array;
    }

    @Override
    public void close() throws Exception {
        if (this.ptr != 0) {
            closeArray(ptr);
            LOGGER.debug("Closed JSArray with id {}", ptr);
            ptr = 0;
        }
    }

}
