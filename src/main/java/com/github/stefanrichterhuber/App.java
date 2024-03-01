package com.github.stefanrichterhuber;

import java.io.IOException;
import java.math.BigDecimal;

import com.github.stefanrichterhuber.quickjs.QuickJSContext;
import com.github.stefanrichterhuber.quickjs.QuickJSRuntime;

/**
 * Hello world!
 *
 */
public class App {

    public static void main(String[] args) throws IOException {
        try (QuickJSRuntime runtime = new QuickJSRuntime();
                QuickJSContext context = runtime.createContext()) {

            {
                context.setGlobal("fa", v -> true);
                context.setGlobal("fb", v -> 3.23d);
                context.setGlobal("fc", v -> 3.23f);
                context.setGlobal("fd", v -> "Hello");
                context.setGlobal("fe", v -> 34);
                context.setGlobal("ff", v -> 34l);
                context.setGlobal("fg", v -> new BigDecimal("123"));

                System.out.println("Result from call: " + context.eval("fa(12)"));

                System.out.println("Result from call: " + context.eval("fb(true)"));
                System.out.println("Result from call: " + context.eval("fc(12.123323)"));
                System.out.println("Result from call: " + context.eval("fd('a')"));
                System.out.println("Result from call: " + context.eval("fe('a')"));
                System.out.println("Result from call: " + context.eval("ff('a')"));
                System.out.println("Result from call: " + context.eval("fg('a')"));
            }
            {
                context.setGlobal("a", "Hello");
                context.setGlobal("b", true);
                context.setGlobal("c", 12);
                context.setGlobal("d", 12.12d);
                context.setGlobal("e", new BigDecimal("123"));
            }
        }
    }
}
