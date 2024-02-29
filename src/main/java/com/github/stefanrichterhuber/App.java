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

            // String result = context.eval("f(a + ' ' + b + '!')"); {
            {
                Object result = context.eval("let f = function() {\n" + //
                        "    return a +b ;\n" + //
                        "  }; f");

                System.out.println(result);

            }
            {
                Object result = context.eval("({a: 'b'})");

                System.out.println(result);

            }
            {
                Object result = context.eval("1+2");

                System.out.println(result);

            }
            {
                Object result = context.eval("1.2 +  2.3 * Math.PI");

                System.out.println(result);

            }
            {
                Object result = context.eval("a + ' ' + b");

                System.out.println(result);

            }
            {
                Object r1 = context.eval("1 == 2");
                Object r2 = context.eval("1 == 1");

                System.out.println(r1 + " " + r2);
            }

        }
    }
}
