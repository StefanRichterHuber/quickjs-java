package com.github.stefanrichterhuber.quickjs;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;

import org.junit.jupiter.api.Test;

public class QuickJSContextTest {
    /**
     * Add and retrieve simple values from the JS context
     * 
     * @throws Exception
     */
    @Test
    public void simpleSetAndGetGlobal() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            // Set and get string
            {
                context.setGlobal("a", "hello");
                Object v = context.getGlobal("a");
                assertInstanceOf(String.class, v);
                assertEquals("hello", v);
            }
            // Set and get int
            {
                context.setGlobal("b", 3);
                Object v = context.getGlobal("b");
                assertInstanceOf(Integer.class, v);
                assertEquals(3, v);
            }
            // Set and get double
            {
                context.setGlobal("c", 3.1415d);
                Object v = context.getGlobal("c");
                assertInstanceOf(Double.class, v);
                assertEquals(3.1415d, v);
            }
            // Set and get boolean
            {
                context.setGlobal("d", false);
                Object v = context.getGlobal("d");
                assertInstanceOf(Boolean.class, v);
                assertEquals(false, v);
            }
            {
                context.setGlobal("e", true);
                Object v = context.getGlobal("e");
                assertInstanceOf(Boolean.class, v);
                assertEquals(true, v);
            }
        }
    }

    /**
     * Add and execute different type of java functions to the JS context
     * 
     * @throws Exception
     */
    @Test
    public void javaFunctionTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            // Supplier
            {
                context.setGlobal("a", () -> "Hello");
                Object v = context.eval("a()");
                assertInstanceOf(String.class, v);
                assertEquals("Hello", v);
            }
            // Function
            {
                context.setGlobal("b", v -> "Hello " + v);
                Object v = context.eval("b('World')");
                assertInstanceOf(String.class, v);
                assertEquals("Hello World", v);
            }
            // BiFunction
            {
                context.setGlobal("c", (a, b) -> a.toString() + " " + b.toString());
                Object v = context.eval("c('Hello', 'World')");
                assertInstanceOf(String.class, v);
                assertEquals("Hello World", v);
            }
            // Consumer
            {
                CompletableFuture<String> result = new CompletableFuture<>();
                context.setGlobal("d", v -> {
                    result.complete(v.toString());
                });
                context.eval("d('Hello')");
                assertEquals("Hello", result.join());
            }
        }
    }

    /**
     * Java functions can accept and return all kind of supported java parameters
     * (including maps and other functions)
     * 
     * @throws Exception
     */
    @Test
    public void javaFunctionParameterTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            // return all kind of simple values
            {
                context.setGlobal("f1", () -> true);
                context.setGlobal("f2", () -> 3);
                context.setGlobal("f3", () -> 3.1223);
                context.setGlobal("f4", () -> "hello");

                assertEquals(true, context.eval("f1()"));
                assertEquals(3, context.eval("f2()"));
                assertEquals(3.1223, context.eval("f3()"));
                assertEquals("hello", context.eval("f4()"));
            }
            // Accepts all kind of simple values
            {
                context.setGlobal("f5", (Boolean v) -> v ? "hello" : "world");
                context.setGlobal("f6", (Integer v) -> v + 2);
                context.setGlobal("f7", (Double v) -> v + 0.2d);
                context.setGlobal("f8", (String v) -> v + " " + "world");

                assertEquals("hello", context.eval("f5(true)"));
                assertEquals("world", context.eval("f5(false)"));
                assertEquals(5, context.eval("f6(3)"));
                // FIXME we receive int value in f7 instead of double
                // assertEquals(3.2223, context.eval("f7(3.1223)"));
                assertEquals("hello world", context.eval("f8('hello')"));
            }
            // returns other functions
            {
                Supplier<String> f1 = () -> "hello";
                Supplier<Supplier<String>> f2 = () -> f1;
                context.setGlobal("f9", f2);
                assertEquals("hello", context.eval("f9()()"));

            }
            // accept JS functions
            {
                Function<QuickJSFunction, String> f1 = f -> {
                    // Closing the function is necessary to prevent memory leaks
                    try (QuickJSFunction f2 = f) {
                        var r = f2.call("hello") + "!";
                        return r;
                    }
                };
                context.setGlobal("f10", f1);
                assertEquals("hello world!", context.eval("let f = function(v) { return v + ' world'; };f10(f)"));
            }
            // Returns maps
            {
                Supplier<Map<String, Object>> f1 = () -> Map.of("a", 1, "b", 2);
                context.setGlobal("f11", f1);
                assertEquals(2, context.eval("f11().b"));
            }
        }
    }

    /**
     * Export functions from JS with different parameters as QuickJSFunction
     */
    @Test
    public void jsFunctionTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            // No parameter
            {
                Object v = context.eval("let a = function() { return 'hello'; }; a");
                assertInstanceOf(QuickJSFunction.class, v);
                try (QuickJSFunction f = (QuickJSFunction) v) {
                    assertEquals("hello", f.call());
                }
            }
            // One parameter
            {
                Object v = context.eval("let b = function(v) { return 'hello ' + v; }; b");
                assertInstanceOf(QuickJSFunction.class, v);
                try (QuickJSFunction f = (QuickJSFunction) v) {
                    assertEquals("hello world", f.call("world"));
                }
            }
            // Two parameters
            {
                Object v = context.eval("let c = function(v1, v2) { return v1 + ' ' + v2; };c");
                assertInstanceOf(QuickJSFunction.class, v);
                try (QuickJSFunction f = (QuickJSFunction) v) {
                    assertEquals("hello world", f.call("hello", "world"));
                }
            }
            // Call multiple times to ensure stability
            {
                Object v = context.eval("let d = function() { return 'hello'; }; d");
                assertInstanceOf(QuickJSFunction.class, v);
                try (QuickJSFunction f = (QuickJSFunction) v) {
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                    assertEquals("hello", f.call());
                }
            }
            // Get a global function
            {
                context.eval("var e = function() { return 'hello'; };");

                Object v = context.getGlobal("e");
                assertInstanceOf(QuickJSFunction.class, v);
                try (QuickJSFunction f = (QuickJSFunction) v) {
                    assertEquals("hello", f.call());
                }
            }

        }
    }

    /**
     * Java object arrays are converted to js arrays
     * 
     * @throws Exception
     */
    @Test
    public void arrayTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            context.setGlobal("vs", new Object[] { "a", "b", "c" });
            Object result = context.eval("vs");
            assertEquals(3, ((List<?>) result).size());
            assertEquals("a", ((List<?>) result).get(0));
            assertEquals("b", ((List<?>) result).get(1));
            assertEquals("c", ((List<?>) result).get(2));
        }
    }

    /**
     * JS arrays are converted to java.util.List and vice versa
     */
    @Test
    public void listTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            {
                Object result = context.eval("[1, 2, 3];");
                assertInstanceOf(List.class, result);
                assertEquals(3, ((List<?>) result).size());
                assertEquals(1, ((List<?>) result).get(0));
                assertEquals(2, ((List<?>) result).get(1));
                assertEquals(3, ((List<?>) result).get(2));
            }

            {
                context.setGlobal("vs", List.of("hello", "world", "!"));
                Object result = context.eval("vs");
                assertInstanceOf(List.class, result);
                assertEquals(3, ((List<?>) result).size());
                assertEquals("hello", ((List<?>) result).get(0));
                assertEquals("world", ((List<?>) result).get(1));
                assertEquals("!", ((List<?>) result).get(2));
            }
        }
    }

    /**
     * Java Maps could be mapped to JS objects. Key type must be string, value
     * supports all supported java types (simple
     * values, functions, nested maps)
     * 
     * @throws Exception
     */
    @Test
    public void javaMapToJSObjectTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            // Simple values in map
            {
                Map<String, Object> m = Map.of("a", 1, "b", 2.7d, "c", true, "d", "hello");
                context.setGlobal("a", m);

                Object a = context.eval("a.a");
                assertEquals(1, a);

                Object b = context.eval("a.b");
                assertEquals(2.7d, b);

                Object c = context.eval("a.c");
                assertEquals(true, c);

                Object d = context.eval("a.d");
                assertEquals("hello", d);
            }
            // Functions could also be stored in a map and called from js
            {
                Function<String, String> f1 = v -> v.repeat(3);
                Supplier<String> f2 = () -> "hello world";
                BiFunction<String, String, String> f3 = (v1, v2) -> v1 + " " + v2;

                Map<String, Object> m = Map.of("a", f1, "b", f2, "c", f3);

                context.setGlobal("b", m);

                Object a = context.eval("b.a");
                assertInstanceOf(QuickJSFunction.class, a);
                try (QuickJSFunction f = (QuickJSFunction) a) {
                    assertEquals("hellohellohello", f.call("hello"));
                }

                Object b = context.eval("b.b");
                assertInstanceOf(QuickJSFunction.class, b);
                try (QuickJSFunction f = (QuickJSFunction) b) {
                    assertEquals("hello world", f.call());
                }

                Object c = context.eval("b.c");
                assertInstanceOf(QuickJSFunction.class, c);
                try (QuickJSFunction f = (QuickJSFunction) c) {
                    assertEquals("hello world", f.call("hello", "world"));
                }
            }

        }
    }

    /**
     * JS objects can be mapped to {@link java.util.Map}. Values could be all
     * supported JS value types including other objects and functions
     * 
     * @throws Exception
     */
    @Test
    @SuppressWarnings("unchecked")
    public void jsObjectToMapTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            // Simple values in map
            {
                Object v = context.eval("({a: 1, b: 2.7, c: true, d: 'hello'})");
                assertNotNull(v);
                assertInstanceOf(Map.class, v);
                Map<String, Object> m = (Map<String, Object>) v;
                assertEquals(1, m.get("a"));
                // FIXME b is not exported as double
                // assertEquals(2.7, m.get("b"));
                assertEquals(true, m.get("c"));
                assertEquals("hello", m.get("d"));
            }
            // Nested maps in maps
            {
                Object v = context.eval("({a: {b: {c: 'Hello'}}})");
                assertNotNull(v);
                assertInstanceOf(Map.class, v);
                Map<String, Object> m = (Map<String, Object>) v;
                Map<String, Object> a = (Map<String, Object>) m.get("a");
                Map<String, Object> b = (Map<String, Object>) a.get("b");
                assertEquals("Hello", b.get("c"));
            }
            // Functions could be part of a map
            {
                Object v = context.eval("({a: function() { return 'hello'; }})");
                assertNotNull(v);
                assertInstanceOf(Map.class, v);
                Map<String, Object> m = (Map<String, Object>) v;
                Object fc = m.get("a");
                assertInstanceOf(QuickJSFunction.class, fc);
                try (QuickJSFunction f = (QuickJSFunction) fc) {
                    assertEquals("hello", f.call());
                }

            }
        }
    }

    @Test
    public void limitRuntimeTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            runtime.withScriptRuntimeLimit(1, TimeUnit.SECONDS);

            long startTime = System.currentTimeMillis();

            try {
                // This never finishes without interruption
                context.eval("while (true) {  }");
                fail("This should never happen, because there is an endless loop before");

            } catch (Exception e) {
                // Expected exception due to interruption
                long endTime = System.currentTimeMillis();
                long duration = endTime - startTime;
                // Should be exactly 1 second, give it some extra time
                assertTrue(duration < 1500);
            }

        }
    }

}
