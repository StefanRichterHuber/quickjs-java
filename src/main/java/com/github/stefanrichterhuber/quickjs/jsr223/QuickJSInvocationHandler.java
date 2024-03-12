package com.github.stefanrichterhuber.quickjs.jsr223;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;

/**
 * Implementation of InvocationHandler for a dynamic proxy which forwards all
 * method invocations into the js engine.
 */
public class QuickJSInvocationHandler implements InvocationHandler {
    private final QuickJSScriptEngine engine;
    private final Object thiz;
    private final String namespace;

    @Override
    public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
        String name = method.getName();
        if (namespace != null) {
            name = namespace + "." + name;
        }

        if (thiz != null) {
            return engine.invokeMethod(thiz, name, args);
        } else {
            return engine.invokeFunction(name, args);
        }

    }

    QuickJSInvocationHandler(QuickJSScriptEngine engine, String namespace, Object thiz) {
        this.engine = engine;
        this.thiz = thiz;
        this.namespace = namespace;
    }

    public QuickJSScriptEngine getEngine() {
        return this.engine;
    }

    @SuppressWarnings("unchecked")
    static <T> T createProxyFor(QuickJSScriptEngine engine, String namespace, Object thiz, Class<T> clazz) {
        return (T) java.lang.reflect.Proxy.newProxyInstance(clazz.getClassLoader(), new Class[] { clazz },
                new QuickJSInvocationHandler(engine, namespace, thiz));
    }

}
