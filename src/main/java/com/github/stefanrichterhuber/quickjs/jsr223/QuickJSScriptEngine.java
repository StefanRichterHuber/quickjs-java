package com.github.stefanrichterhuber.quickjs.jsr223;

import java.io.IOException;
import java.io.Reader;
import java.util.concurrent.TimeUnit;

import javax.script.Bindings;
import javax.script.Invocable;
import javax.script.ScriptContext;
import javax.script.ScriptEngine;
import javax.script.ScriptEngineFactory;
import javax.script.ScriptException;

import com.github.stefanrichterhuber.quickjs.QuickJSContext;
import com.github.stefanrichterhuber.quickjs.QuickJSRuntime;

public class QuickJSScriptEngine implements ScriptEngine, Invocable {
    public static final String TIMEOUT = "com.github.stefanrichterhuber.quickjs.timeout";
    public static final String MEMORY_LIMT = "com.github.stefanrichterhuber.quickjs.memoryLimit";

    private final QuickJSScriptEngineFactory factory;
    private final QuickJSRuntime runtime;
    private ScriptContext context;

    QuickJSScriptEngine(QuickJSScriptEngineFactory factory) {
        this.factory = factory;
        this.runtime = new QuickJSRuntime();
        this.context = new QuickJSScriptContext(null);
    }

    private static String readReader(Reader reader) throws IOException {
        StringBuilder sb = new StringBuilder();
        char[] buffer = new char[1024];
        int read;
        while ((read = reader.read(buffer)) != -1) {
            sb.append(buffer, 0, read);
        }
        return sb.toString();
    }

    @Override
    public Object eval(String script, ScriptContext context) throws ScriptException {
        return eval(script, context, new QuickJSScriptBindings());
    }

    public Object eval(String script, ScriptContext context, Bindings bindings) throws ScriptException {
        QuickJSScriptContext ctx = null;
        if (context instanceof QuickJSScriptContext) {
            ctx = (QuickJSScriptContext) context;
        } else {
            ctx = new QuickJSScriptContext(context);
        }

        try (QuickJSContext qjs = runtime.createContext()) {
            ctx.addToQuickJSContext(qjs);
            QuickJSScriptContext.addToQuickJSContext(qjs, bindings);

            return qjs.eval(script);
        } catch (Exception e) {
            throw new ScriptException(e);
        }
    }

    @Override
    public Object eval(Reader reader, ScriptContext context) throws ScriptException {
        try {
            String script = readReader(reader);
            return eval(script, context);
        } catch (IOException e) {
            throw new ScriptException(e);
        }
    }

    @Override
    public Object eval(String script) throws ScriptException {
        return eval(script, this.getContext());
    }

    @Override
    public Object eval(Reader reader) throws ScriptException {
        try {
            String script = readReader(reader);
            return eval(script);
        } catch (IOException e) {
            throw new ScriptException(e);
        }
    }

    @Override
    public Object eval(String script, Bindings n) throws ScriptException {
        return eval(script, getContext(), new QuickJSScriptBindings());
    }

    @Override
    public Object eval(Reader reader, Bindings n) throws ScriptException {
        try {
            String script = readReader(reader);
            return eval(script, n);
        } catch (IOException e) {
            throw new ScriptException(e);
        }
    }

    @Override
    public void put(String key, Object value) {
        if (key.equals(TIMEOUT)) {
            this.runtime.withScriptRuntimeLimit((long) value, TimeUnit.MILLISECONDS);
        } else if (key.equals(MEMORY_LIMT)) {
            this.runtime.withMemoryLimit((long) value);
        }
        getContext().setAttribute(key, value, ScriptContext.ENGINE_SCOPE);
    }

    @Override
    public Object get(String key) {
        return getContext().getAttribute(key, ScriptContext.ENGINE_SCOPE);
    }

    @Override
    public Bindings getBindings(int scope) {
        return getContext().getBindings(scope);
    }

    @Override
    public void setBindings(Bindings bindings, int scope) {
        // TODO
        getContext().setBindings(bindings, scope);
    }

    @Override
    public Bindings createBindings() {
        return new QuickJSScriptBindings();
    }

    @Override
    public ScriptContext getContext() {
        return this.context != null ? this.context : new QuickJSScriptContext(null);
    }

    @Override
    public void setContext(ScriptContext context) {
        if (context instanceof QuickJSScriptContext) {
            this.context = context;
        } else {
            this.context = new QuickJSScriptContext(context);
        }
    }

    @Override
    public ScriptEngineFactory getFactory() {
        return this.factory;
    }

    @Override
    public Object invokeMethod(Object thiz, String name, Object... args) throws ScriptException, NoSuchMethodException {
        // TODO todo implement some handling on 'thiz'
        throw new UnsupportedOperationException("Unimplemented method 'invokeMethod'");
    }

    @Override
    public Object invokeFunction(String name, Object... args) throws ScriptException, NoSuchMethodException {
        QuickJSScriptContext ctx = null;
        if (context instanceof QuickJSScriptContext) {
            ctx = (QuickJSScriptContext) context;
        } else {
            ctx = new QuickJSScriptContext(context);
        }

        try (QuickJSContext qjs = runtime.createContext()) {
            ctx.addToQuickJSContext(qjs);
            return qjs.invoke(name, args);
        } catch (Exception e) {
            throw new ScriptException(e);
        }
    }

    @Override
    public <T> T getInterface(Class<T> clasz) {
        return QuickJSInvocationHandler.createProxyFor(this, null, null, clasz);
    }

    @Override
    public <T> T getInterface(Object thiz, Class<T> clasz) {
        return QuickJSInvocationHandler.createProxyFor(this, null, thiz, clasz);
    }

}
