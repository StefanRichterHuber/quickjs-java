package com.github.stefanrichterhuber.quickjs.jsr223;

import java.io.Reader;
import java.io.Writer;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.BiFunction;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Supplier;

import javax.script.Bindings;
import javax.script.ScriptContext;

import com.github.stefanrichterhuber.quickjs.QuickJSContext;
import com.github.stefanrichterhuber.quickjs.QuickJSFunction;

public class QuickJSScriptContext implements ScriptContext {
    private final ScriptContext delegate;
    private final Map<Integer, Bindings> bindings = new HashMap<>();
    private Writer writer;
    private Writer errorWriter;
    private Reader reader;

    public QuickJSScriptContext(ScriptContext delegate) {
        this.delegate = delegate;
    }

    public ScriptContext getDelegate() {
        return this.delegate;
    }

    public void addToQuickJSContext(QuickJSContext ctx) {
        if (this.getDelegate() != null) {
            for (int scope : getScopes().reversed()) {
                addToQuickJSContext(ctx, this.getDelegate().getBindings(scope));
            }
        } else {
            for (int scope : getScopes().reversed()) {
                addToQuickJSContext(ctx, bindings.get(scope));
            }
        }
        // TODO add support for writer, errorWriter and reader
    }

    @SuppressWarnings({ "unchecked", "rawtypes" })
    static void addToQuickJSContext(QuickJSContext ctx, Map<String, Object> values) {
        if (values != null) {
            for (Map.Entry<String, Object> entry : values.entrySet()) {
                if (entry.getValue() instanceof Iterable) {
                    ctx.setGlobal(entry.getKey(), (List<?>) entry.getValue());
                } else if (entry.getValue() instanceof Map) {
                    ctx.setGlobal(entry.getKey(), (Map<String, Object>) entry.getValue());
                } else if (entry.getValue() instanceof Double) {
                    ctx.setGlobal(entry.getKey(), (Double) entry.getValue());
                } else if (entry.getValue() instanceof Float) {
                    ctx.setGlobal(entry.getKey(), (Float) entry.getValue());
                } else if (entry.getValue() instanceof Integer) {
                    ctx.setGlobal(entry.getKey(), (Integer) entry.getValue());
                } else if (entry.getValue() instanceof String) {
                    ctx.setGlobal(entry.getKey(), (String) entry.getValue());
                } else if (entry.getValue() instanceof Supplier) {
                    ctx.setGlobal(entry.getKey(), (Supplier) entry.getValue());
                } else if (entry.getValue() instanceof Consumer) {
                    ctx.setGlobal(entry.getKey(), (Consumer) entry.getValue());
                } else if (entry.getValue() instanceof Function) {
                    ctx.setGlobal(entry.getKey(), (Function) entry.getValue());
                } else if (entry.getValue() instanceof BiFunction) {
                    ctx.setGlobal(entry.getKey(), (BiFunction) entry.getValue());
                } else if (entry.getValue() instanceof Boolean) {
                    ctx.setGlobal(entry.getKey(), (Boolean) entry.getValue());
                } else if (entry.getValue() instanceof QuickJSFunction) {
                    ctx.setGlobal(entry.getKey(), (QuickJSFunction) entry.getValue());
                }
            }
        }
    }

    @Override
    public void setBindings(Bindings bindings, int scope) {
        if (this.delegate != null) {
            this.delegate.setBindings(bindings, scope);
        } else {
            this.bindings.put(scope, bindings);
        }
    }

    @Override
    public Bindings getBindings(int scope) {
        if (this.delegate != null) {
            return this.delegate.getBindings(scope);
        } else {
            return this.bindings.get(scope);
        }
    }

    @Override
    public void setAttribute(String name, Object value, int scope) {
        getBindings(scope).put(name, value);
    }

    @Override
    public Object getAttribute(String name, int scope) {
        return getBindings(scope).get(name);
    }

    @Override
    public Object removeAttribute(String name, int scope) {
        return getBindings(scope).remove(name);
    }

    @Override
    public Object getAttribute(String name) {
        for (int scope : getScopes()) {
            Object value = getAttribute(name, scope);
            if (value != null) {
                return value;
            }
        }
        return null;

    }

    @Override
    public int getAttributesScope(String name) {
        for (int scope : getScopes()) {
            Object value = getAttribute(name, scope);
            if (value != null) {
                return scope;
            }
        }
        return -1;
    }

    @Override
    public Writer getWriter() {
        return this.writer;
    }

    @Override
    public Writer getErrorWriter() {
        return this.errorWriter;
    }

    @Override
    public void setWriter(Writer writer) {
        this.writer = writer;
    }

    @Override
    public void setErrorWriter(Writer writer) {
        this.errorWriter = writer;
    }

    @Override
    public Reader getReader() {
        return this.reader;
    }

    @Override
    public void setReader(Reader reader) {
        this.reader = reader;
    }

    @Override
    public List<Integer> getScopes() {
        return List.of(ScriptContext.ENGINE_SCOPE, ScriptContext.GLOBAL_SCOPE);
    }

}
