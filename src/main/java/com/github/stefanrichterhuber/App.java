package com.github.stefanrichterhuber;

import com.github.stefanrichterhuber.quickjs.QuickJSContext;
import com.github.stefanrichterhuber.quickjs.QuickJSFunction;
import com.github.stefanrichterhuber.quickjs.QuickJSRuntime;

/**
 * Hello world!
 * 
 */
public class App {

    public static void main(String[] args) throws Exception {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            {
                QuickJSFunction f = (QuickJSFunction) context.eval("let a = function() { return 'hello'; };\n a");
                Object r0 = f.call();
                Object r1 = f.call();
                System.out.println(r1);
            }

            {
                QuickJSFunction f = (QuickJSFunction) context.eval("let b = function(c) { return 'hello ' + c; };\n b");
                Object r0 = f.call("world");
                Object r1 = f.call("all world");
                System.out.println(r1);
            }

            // {
            // context.setGlobal("m",
            // Map.of("f1", (Supplier<String>) () -> "no mans sky", "hello", "world"));
            // context.setGlobal("fa", () -> "Hello");
            // context.setGlobal("fb", a -> a);
            // context.setGlobal("fc", (a, b) -> a.toString() + b.toString());
            // context.setGlobal("fd", a -> {
            // System.out.println(a);
            // });

            // System.out.println("Result from accessing map m: " +
            // context.eval("m.hello"));
            // System.out.println("Result from accessing map function m: " +
            // context.eval("m.f1()"));
            // System.out.println("Result from call fa: " + context.eval("fa()"));
            // System.out.println("Result from call fb: " + context.eval("fb('Hello')"));
            // System.out.println("Result from call fc: " + context.eval("fc('Hello',
            // 'World')"));
            // System.out.println("Result from call fd: " + context.eval("fd('Hello')"));
            // }

            // {
            // context.setGlobal("fa", v -> true);
            // context.setGlobal("fb", v -> 3.23d);
            // context.setGlobal("fc", v -> 3.23f);
            // context.setGlobal("fd", v -> "Hello");
            // context.setGlobal("fe", v -> 34);
            // // context.setGlobal("ff", v -> BigInteger.valueOf(234l));
            // // context.setGlobal("ff", v -> 34l);
            // // context.setGlobal("fg", v -> new BigDecimal("123"));
            // context.setGlobal("fh", v -> ((Function<String, String>) w -> "Hello
            // inception"));

            // System.out.println("Result from call fa: " + context.eval("fa(12)"));
            // System.out.println("Result from call fb: " + context.eval("fb(true)"));
            // System.out.println("Result from call fc: " + context.eval("fc(12.123323)"));
            // System.out.println("Result from call fd: " + context.eval("fd('a')"));
            // System.out.println("Result from call fe: " + context.eval("fe('a')"));
            // // System.out.println("Result from call ff: " + context.eval("ff('a')"));
            // // System.out.println("Result from call fg: " + context.eval("fg('a')"));
            // System.out.println("Result from call fh: " + context.eval("fh('a')('b')"));
            // }

            // {
            // context.setGlobal("f", BigInteger.valueOf(1234l));
            // context.setGlobal("e", new BigDecimal("123"));
            // context.setGlobal("a", "Hello");
            // context.setGlobal("b", true);
            // context.setGlobal("c", 12);
            // context.setGlobal("d", 12.12d);
            // }
        }
    }
}
