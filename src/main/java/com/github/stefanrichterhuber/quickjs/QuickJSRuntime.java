package com.github.stefanrichterhuber.quickjs;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

import io.questdb.jar.jni.JarJniLoader;

public class QuickJSRuntime implements AutoCloseable {
    private static final Logger LOGGER = LogManager.getLogger();

    static {
        JarJniLoader.loadLib(
                QuickJSRuntime.class,
                // A platform-specific path is automatically suffixed to path below.
                "/libs",
                // The "lib" prefix and ".so|.dynlib|.dll" suffix are added automatically as
                // needed.
                "quickjslib");
    }

    private long ptr;

    private native long createRuntime();

    private native void closeRuntime(long ptr);

    private native void initLogging();

    /**
     * This method is called by the native code to log a message.
     * 
     * @param level
     * @param message
     */
    static void runtimeLog(int level, String message) {
        switch (level) {
            case 0:
                LOGGER.trace(message);
                break;
            case 1:
                LOGGER.debug(message);
                break;
            case 2:
                LOGGER.info(message);
                break;
            case 3:
                LOGGER.warn(message);
                break;
            case 4:
                LOGGER.error(message);
                break;
            default:
                LOGGER.error("Unknown log level " + level + ": " + message);
        }
    }

    public QuickJSRuntime() {
        initLogging();
        ptr = createRuntime();
    }

    long getRuntimePointer() {
        if (ptr == 0) {
            throw new IllegalStateException("QuickJSRuntime closed");
        }
        return ptr;
    }

    @Override
    public void close() throws RuntimeException {
        if (ptr != 0) {
            closeRuntime(ptr);
            ptr = 0;
        }
    }

    public QuickJSContext createContext() {
        return new QuickJSContext(this);
    }

}
