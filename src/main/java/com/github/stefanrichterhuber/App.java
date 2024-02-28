package com.github.stefanrichterhuber;

import java.util.function.Function;

import io.questdb.jar.jni.JarJniLoader;

/**
 * Hello world!
 *
 */
public class App {

    public static native String executeScript(String script);

    public static native void setGlobal(String name, int value);

    public static native void setGlobal(String name, String value);

    public static native void setGlobal(String name, Function<String, String> f);

    public static native void init();

    public static void main(String[] args) {
        // System.load("/home/stefan/Dokumente/quickjs-java/src/main/resources/libs/linux-x86-64/libquickjslib.so");

        JarJniLoader.loadLib(
                App.class,

                // A platform-specific path is automatically suffixed to path below.
                "/libs",

                // The "lib" prefix and ".so|.dynlib|.dll" suffix are added automatically as
                // needed.
                "quickjslib");

        init();

        {
            setGlobal("a", 5);
            setGlobal("b", 10);

            String result = executeScript("a + b + 3");
            System.out.println(result);
        }

        {
            setGlobal("a", "Hello");
            setGlobal("b", "World");

            String result = executeScript("a + ' '+  b ");
            System.out.println(result);
        }

        {
            setGlobal("a", "Hello");
            setGlobal("b", "World");
            setGlobal("f", v -> v + " " + v + " " + v);

            String result = executeScript("f(a)");
            System.out.println(result);
        }
    }
}
