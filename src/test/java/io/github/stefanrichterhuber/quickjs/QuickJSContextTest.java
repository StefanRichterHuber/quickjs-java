package io.github.stefanrichterhuber.quickjs;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.stream.Collectors;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

public class QuickJSContextTest {
    private static final Logger LOGGER = LogManager.getLogger();

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
     * JS eval can return values of all supported types
     * 
     * @throws Exception
     */
    @Test
    public void getResultFromEval() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {
            // Get double from eval
            {
                Object v = context.eval("2.74");
                assertInstanceOf(Double.class, v);
                assertEquals(2.74d, (Double) v, 0.001d);
            }
            // Get int from eval
            {
                Object v = context.eval("2");
                assertInstanceOf(Integer.class, v);
                assertEquals(2, (Integer) v);
            }
            // Get String from eval
            {
                Object v = context.eval("'2'");
                assertInstanceOf(String.class, v);
                assertEquals("2", (String) v);
            }
            // Get Boolean from eval
            {
                Object v = context.eval("true");
                assertInstanceOf(Boolean.class, v);
                assertTrue((Boolean) v);
            }
            {
                Object v = context.eval("false");
                assertInstanceOf(Boolean.class, v);
                assertFalse((Boolean) v);
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
                context.setGlobal("b", (Function<String, String>) v -> "Hello " + v);
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
            // BiConsumer
            {
                CompletableFuture<String> result = new CompletableFuture<>();
                context.setGlobal("e", (a, b) -> {
                    result.complete(a.toString() + " " + b.toString());
                });
                context.eval("e('Hello', 'World')");
                assertEquals("Hello World", result.join());
            }
        }
    }

    /**
     * VariadicFunction is the fall-back solution for all mapping for function calls
     * not being matchable by the standard java.util.function package.
     */
    @Test
    public void variadicFunctionTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            context.setGlobal("a", (VariadicFunction<Object>) (Object... args) -> {
                StringBuilder sb = new StringBuilder();
                for (Object s : args) {
                    sb.append(s);
                }
                return sb.toString();
            });

            assertEquals("Hello World", context.eval("a('Hello',' ', 'World')"));
        }
    }

    /**
     * JS Exceptions are mapped to QuickJSScriptException
     * 
     * @throws Exception
     */
    @Test
    public void jsExceptionTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            context.eval("\n\nthrow new Error('Things happened')");
            fail("Should not reach this point");
        } catch (Exception e) {
            assertInstanceOf(QuickJSScriptException.class, e);
            assertEquals("Things happened", e.getMessage());

            assertEquals(3, ((QuickJSScriptException) e).getLineNumber());
            assertEquals("<script>", ((QuickJSScriptException) e).getFileName());
        }
    }

    /**
     * Java Exceptions are mapped to QuickJSScriptException with correct filename
     * and line-number, but have the original
     * java exception as cause. The original stacktrace, however, is lost.
     * 
     * @throws Exception
     */
    @Test
    public void javaExceptionTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            Function<String, String> a = v -> {
                throw new IllegalArgumentException("Things happened");
            };
            context.setGlobal("a", a);

            context.eval("a('hello')");
            fail("Should not reach this point");
        } catch (Exception e) {
            assertInstanceOf(QuickJSScriptException.class, e);
            assertEquals("Things happened", e.getMessage());
            assertEquals("QuickJSContextTest.java", ((QuickJSScriptException) e).getFileName());

            var cause = e.getCause();
            assertInstanceOf(IllegalArgumentException.class, cause);
            assertEquals("Things happened", cause.getMessage());
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
                    QuickJSFunction f2 = f;
                    var r = f2.apply("hello") + "!";
                    return r;
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
                QuickJSFunction f = (QuickJSFunction) v;
                assertEquals("hello", f.apply());

            }
            // One parameter
            {
                Object v = context.eval("let b = function(v) { return 'hello ' + v; }; b");
                assertInstanceOf(QuickJSFunction.class, v);
                QuickJSFunction f = (QuickJSFunction) v;
                assertEquals("hello world", f.apply("world"));
            }
            // Two parameters
            {
                Object v = context.eval("let c = function(v1, v2) { return v1 + ' ' + v2; };c");
                assertInstanceOf(QuickJSFunction.class, v);
                QuickJSFunction f = (QuickJSFunction) v;
                assertEquals("hello world", f.apply("hello", "world"));
            }
            // Call multiple times to ensure stability
            {
                Object v = context.eval("let d = function() { return 'hello'; }; d");
                assertInstanceOf(QuickJSFunction.class, v);
                QuickJSFunction f = (QuickJSFunction) v;
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
                assertEquals("hello", f.apply());
            }
            // Get a global function
            {
                context.eval("var e = function() { return 'hello'; };");

                Object v = context.getGlobal("e");
                assertInstanceOf(QuickJSFunction.class, v);
                QuickJSFunction f = (QuickJSFunction) v;
                assertEquals("hello", f.apply());
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

    @Test
    public void sublistTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            Object result = context.eval("[1, 2, 3 ,4 ,5, 6, 7];");
            assertInstanceOf(List.class, result);
            assertEquals(7, ((List<?>) result).size());
            assertEquals(1, ((List<?>) result).get(0));
            assertEquals(2, ((List<?>) result).get(1));
            assertEquals(3, ((List<?>) result).get(2));
            assertEquals(4, ((List<?>) result).get(3));
            assertEquals(5, ((List<?>) result).get(4));
            assertEquals(6, ((List<?>) result).get(5));
            assertEquals(7, ((List<?>) result).get(6));

            // First sublist
            List<?> sublist = ((List<?>) result).subList(2, 5);
            assertEquals(3, sublist.size());
            assertEquals(3, sublist.get(0));
            assertEquals(4, sublist.get(1));
            assertEquals(5, sublist.get(2));
            String s = sublist.toString();
            assertEquals("[3, 4, 5]", s);

            // Sub list of sublist
            List<?> subsublist = sublist.subList(1, 2);
            assertEquals(1, subsublist.size());
            assertEquals(4, subsublist.get(0));
            String s2 = subsublist.toString();
            assertEquals("[4]", s2);
        }
    }

    /**
     * Creates an object on the js side and modifies it from the java side
     * 
     * @throws Exception
     */
    @SuppressWarnings("resource")
    @Test
    public void jsObjectModificatonTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            Object result = context.eval("let a = {x: 1, y: 2}; a");
            assertInstanceOf(QuickJSObject.class, result);
            QuickJSObject<String, Integer> object = (QuickJSObject<String, Integer>) result;
            assertEquals(2, object.size());
            assertEquals(1, object.get("x"));
            assertEquals(2, object.get("y"));
            assertTrue(object.containsKey("x"));
            assertTrue(object.containsKey("y"));
            assertFalse(object.containsKey("z"));

            object.put("z", 3);
            assertEquals(3, object.size());
            assertEquals(3, context.eval("a.z"));

            context.eval("delete a.x;");
            assertFalse(object.containsKey("x"));

        }
    }

    @Test
    public void javaObjectModificatonTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            QuickJSObject<String, Integer> object = new QuickJSObject<>(context);
            object.put("x", 1);
            object.put("y", 2);
            assertEquals(2, object.size());
            assertEquals(1, object.get("x"));
            assertEquals(2, object.get("y"));
            assertTrue(object.containsKey("x"));
            assertTrue(object.containsKey("y"));
            assertFalse(object.containsKey("z"));

            context.setGlobal("a", object);
            assertEquals(1, context.eval("a.x"));
            assertEquals(2, context.eval("a.y"));

            context.eval("a.z = 3;");
            assertEquals(3, object.get("z"));

            object.remove("y");
            assertFalse(object.containsKey("y"));

        }
    }

    /**
     * Creates a list on the js side and modifies it from the java side
     * 
     * @throws Exception
     */
    @Test
    public void jsListModificationTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            Object result = context.eval("let a = [1, 2, 3]; a");
            assertInstanceOf(List.class, result);
            List<Integer> list = (List<Integer>) result;
            assertEquals(3, list.size());
            assertEquals(1, list.get(0));
            assertEquals(2, list.get(1));
            assertEquals(3, list.get(2));

            list.add(4);
            assertEquals(4, context.eval("a.length"));
            assertEquals(4, context.eval("a[3]"));
            assertEquals(4, list.get(3));

            list.remove(0);
            assertEquals(3, context.eval("a.length"));
            assertEquals(2, context.eval("a[0]"));
            assertEquals(2, list.get(0));

            list.add(1, 99);
            assertEquals(4, context.eval("a.length"));
            assertEquals(99, context.eval("a[1]"));
            assertEquals(99, list.get(1));
        }
    }

    /**
     * Creates a list on the java side and modifies it from the js side
     * 
     * @throws Exception
     */
    @Test
    public void javaListModificationTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            QuickJSArray<Integer> array = new QuickJSArray<>(context);

            array.add(1);
            array.add(2);
            array.add(3);
            context.setGlobal("a", array);

            assertEquals(3, context.eval("a.length"));
            assertEquals(1, context.eval("a[0]"));
            assertEquals(2, context.eval("a[1]"));
            assertEquals(3, context.eval("a[2]"));

            array.set(0, 4);
            assertEquals(4, context.eval("a[0]"));

            array.add(5);
            assertEquals(4, context.eval("a.length"));
            assertEquals(5, context.eval("a[3]"));

            // Array modified in js
            assertEquals(99, context.eval("a[0] = 99"));
            assertEquals(99, array.get(0));

        }

    }

    @Test
    public void nestedListTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {
            Object result = context.eval("let a = ['a', 21, ['x', 'y', 'z']]; a");
            assertInstanceOf(List.class, result);
            assertEquals(3, ((List<?>) result).size());
            assertEquals("a", ((List<?>) result).get(0));
            assertEquals(21, ((List<?>) result).get(1));
            assertEquals(List.of("x", "y", "z"), ((List<?>) result).get(2));
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
                {
                    Object a = context.eval("b.a");
                    assertInstanceOf(QuickJSFunction.class, a);
                    QuickJSFunction f = (QuickJSFunction) a;
                    assertEquals("hellohellohello", f.apply("hello"));
                }
                {
                    Object b = context.eval("b.b");
                    assertInstanceOf(QuickJSFunction.class, b);
                    QuickJSFunction f = (QuickJSFunction) b;
                    assertEquals("hello world", f.apply());
                }
                {
                    Object c = context.eval("b.c");
                    assertInstanceOf(QuickJSFunction.class, c);
                    QuickJSFunction f = (QuickJSFunction) c;
                    assertEquals("hello world", f.apply("hello", "world"));
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
            // Simple nested maps
            {
                Object v = context.eval("({a: {c: 'Hello'}})");
                assertNotNull(v);
                assertInstanceOf(Map.class, v);
                Map<String, Object> m = (Map<String, Object>) v;
                Map<String, Object> a = (Map<String, Object>) m.get("a");
                assertEquals("Hello", a.get("c"));
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
                QuickJSFunction f = (QuickJSFunction) fc;
                assertEquals("hello", f.apply());

            }
        }
    }

    /**
     * Runtime of any script execution can be limited in the QuickJSRuntime object
     * 
     * @throws Exception
     */
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
                assertTrue(e.getMessage().contains("interrupted"));
                // Expected exception due to interruption
                long endTime = System.currentTimeMillis();
                long duration = endTime - startTime;
                // Should be exactly 1 second, give it some extra time
                assertTrue(duration < 1500);
            }

        }
    }

    /**
     * One could directly eval direct(!) byte buffers. This allows to transfer of
     * huge scripts directly into the native context without copy operation.
     */
    @Test
    public void evalByteBufferTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            byte[] script = "3 + 4 + 7".getBytes(StandardCharsets.UTF_8);
            ByteBuffer bb = ByteBuffer.allocateDirect(script.length).put(script);
            Object result = context.eval(bb);

            assertEquals(14, result);
        }
    }

    /**
     * Memory consumption of any script execution can be limited in the
     * QuickJSRuntime object
     * 
     * @throws Exception
     */
    @Test
    @Disabled("Takes too long")
    public void limitMemoryTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            // We need a huge memory limit just for the callback, and some bytes extra for
            // the array
            runtime.withMemoryLimit(90000);

            AtomicInteger ai = new AtomicInteger(0);

            context.setGlobal("f", (List<Integer> l) -> {
                // Log the size of the array
                ai.set(l.size());
            });

            try {

                // This never finishes without hitting memory limit
                context.eval(
                        "const nrs = []; while (true) { nrs.push(5); f(nrs); }");
                fail("This should never happen, because there is an endless loop before");

            } catch (Exception e) {
                assertTrue(e.getMessage().contains("out of memory"));
                // Eventually we arrive here
                assertTrue(ai.get() < 2000);
            }
        }
    }

    /**
     * One can call functions from java, both java functions passed to the script
     * context as well as script native functions
     */
    @Test
    public void invokeFunctionTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            context.setGlobal("f1", (String a) -> "Hello " + a);
            context.eval("function f2(a) { return 'Hello from JS dear ' + a; };");

            String r1 = (String) context.invoke("f1", "World");
            assertEquals("Hello World", r1);

            String r2 = (String) context.invoke("f2", "World");
            assertEquals("Hello from JS dear World", r2);

        }
    }

    public static interface TestInterface {
        String f1(String name);

        String f2(String name);

        default String f3(String name) {
            return "Hello from a default method dear " + name;
        }
    }

    /**
     * Interfaces could be proxied using Java dynamic proxies. All, but default,
     * methods are proxied to the script environment. For method return types and
     * parameter types, again all supported java types are supported. This allows
     * for type-safe invocation of js functions.
     * 
     * @throws Exception
     */
    @Test
    public void proxyTest() throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            context.setGlobal("f1", (String a) -> "Hello " + a);
            context.eval("function f2(a) { return 'Hello from JS dear ' + a; };");

            TestInterface ti = context.getInterface(null, TestInterface.class);

            // Call wrapped java function
            String r1 = ti.f1("World");
            assertEquals("Hello World", r1);

            // Call native js function
            String r2 = ti.f2("World");
            assertEquals("Hello from JS dear World", r2);

            // Call default function
            String r3 = ti.f3("World");
            assertEquals("Hello from a default method dear World", r3);

        }
    }

    public static class TestClass {
        public void call(String name) {
            LOGGER.debug(name);
        }

        public String f1(String name) {
            return "Hello " + name;
        }

        public String f2(String a, String b) {
            return a + " " + b;
        }

        public String f3(String a, String b, String c) {
            return a + " " + b + " " + c;
        }

        public String f4(List<String> args) {
            return args.stream().collect(Collectors.joining(" "));
        }

        @Override
        public String toString() {
            return "TestClass";
        }
    }

    /**
     * Test the utility method createMapOf which provides a rather incomplete
     * mapping of generic objects into Maps of functions.
     * 
     * @throws Exception
     */
    @Test
    public void objectMapTest() throws Exception {

        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            Map<String, Object> m = QuickJSUtils.createMapOf(new TestClass());
            assertNotNull(m);

            context.setGlobal("tc", m);

            Object r1 = context.eval("tc.f1('World')");
            assertEquals("Hello World", r1);

            Object r2 = context.eval("tc.toString()");
            assertEquals("TestClass", r2);

            Object r3 = context.eval("tc.hashCode()");
            assertInstanceOf(Integer.class, r3);

            context.invoke("tc.call", "World");

            try {
                context.invoke("tx.call", "World");
                fail();
            } catch (Exception e) {
                // Expected since, tx does not exist
            }

            Object r4 = context.invoke("tc.f2", "Hello", "World");
            assertEquals("Hello World", r4);

            Object r5 = context.invoke("tc.f3", "Hello", "World", "!");
            assertEquals("Hello World !", r5);

            Object r6 = context.invoke("tc.f4", List.of("Hello", "World", "!"));
            assertEquals("Hello World !", r6);

            // JS arrays are always converted to java.util.List
            Object r7 = context.eval("tc.f4(['Hello', 'World', '!'])");
            assertEquals("Hello World !", r7);
        }
    }
}
