package com.github.stefanrichterhuber;

import java.io.IOException;

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

            context.setGlobal("a", "Hello");
            context.setGlobal("b", "World");
            context.setGlobal("f", v -> v.repeat(3));

            String result = context.eval("f(a + ' ' + b + '!')");

            System.out.println(result);
        }
    }
}
