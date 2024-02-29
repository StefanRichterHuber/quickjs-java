package com.github.stefanrichterhuber.quickjs;

import java.io.Closeable;
import java.io.IOException;

import com.github.stefanrichterhuber.App;

import io.questdb.jar.jni.JarJniLoader;

public class QuickJSRuntime implements Closeable {
    static {
        JarJniLoader.loadLib(
                App.class,
                // A platform-specific path is automatically suffixed to path below.
                "/libs",
                // The "lib" prefix and ".so|.dynlib|.dll" suffix are added automatically as
                // needed.
                "quickjslib");
    }

    private long ptr;

    private native long createRuntime();

    private native void closeRuntime(long ptr);

    public QuickJSRuntime() {
        ptr = createRuntime();
    }

    long getRuntimePointer() {
        if (ptr == 0) {
            throw new IllegalStateException("QuickJSRuntime closed");
        }
        return ptr;
    }

    @Override
    public void close() throws IOException {
        if (ptr != 0) {
            closeRuntime(ptr);
            ptr = 0;
        }
    }

    public QuickJSContext createContext() {
        return new QuickJSContext(this);
    }

}
