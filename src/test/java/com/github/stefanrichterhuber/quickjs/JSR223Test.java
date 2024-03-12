package com.github.stefanrichterhuber.quickjs;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import javax.script.Invocable;
import javax.script.ScriptEngine;
import javax.script.ScriptEngineManager;

import org.junit.jupiter.api.Test;

public class JSR223Test {

    public static interface InvocableTest {
        public String f1(String a);

        public String f2(String a);
    }

    @Test
    public void scriptEngineTest() {
        ScriptEngine eng = new ScriptEngineManager().getEngineByName("QuickJS");
        assertNotNull(eng);

        Invocable i = (Invocable) eng;
        InvocableTest it = i.getInterface(InvocableTest.class);
        assertNotNull(it);

        // context.setGlobal("f1", (String a) -> "Hello " + a);
        // context.eval("function f2(a) { return 'Hello from JS dear ' + a; };");

        String r1 = it.f1("World");
        assertEquals("Hello World", r1);

        String r2 = it.f2("World");
        assertEquals("Hello from JS dear World", r2);

    }

}
