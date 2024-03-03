package com.github.stefanrichterhuber.quickjs;

import java.lang.ref.Cleaner;
import java.util.HashSet;
import java.util.Set;

import org.apache.logging.log4j.Level;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

import io.questdb.jar.jni.JarJniLoader;

public class QuickJSRuntime implements AutoCloseable {
    static final Cleaner CLEANER = Cleaner.create();

    private static final Logger LOGGER = LogManager.getLogger();
    private static final Logger NATIVE_LOGGER = LogManager.getLogger("[QuickJS native library]");

    static {
        JarJniLoader.loadLib(
                QuickJSRuntime.class,
                // A platform-specific path is automatically suffixed to path below.
                "/libs",
                // The "lib" prefix and ".so|.dynlib|.dll" suffix are added automatically as
                // needed.
                "quickjslib");

        if (LOGGER.getLevel() == Level.ERROR || LOGGER.getLevel() == Level.FATAL) {
            initLogging(1);
        } else if (LOGGER.getLevel() == Level.WARN) {
            initLogging(2);
        } else if (LOGGER.getLevel() == Level.INFO) {
            initLogging(3);
        } else if (LOGGER.getLevel() == Level.DEBUG) {
            initLogging(4);
        } else if (LOGGER.getLevel() == Level.TRACE) {
            initLogging(5);
        } else if (LOGGER.getLevel() == Level.OFF) {
            initLogging(0);
        } else {
            LOGGER.warn("Unknown log level " + LOGGER.getLevel() + " , using INFO for native library");
            initLogging(3);
        }
    }

    private long ptr;

    private native long createRuntime();

    private static native void closeRuntime(long ptr);

    private static native void initLogging(int level);

    /**
     * Keep a reference to all contexts created to prevent memory leaks which
     * results in errors when closing the runtime
     */
    private final Set<AutoCloseable> dependedResources = new HashSet<>();

    /**
     * This method is called by the native code to log a message.
     * 
     * @param level
     * @param message
     */
    static void runtimeLog(int level, String message) {
        switch (level) {
            case 0:
                // DO nothing -> log is off
                break;
            case 5:
                NATIVE_LOGGER.trace(message);
                break;
            case 4:
                NATIVE_LOGGER.debug(message);
                break;
            case 3:
                NATIVE_LOGGER.info(message);
                break;
            case 2:
                NATIVE_LOGGER.warn(message);
                break;
            case 1:
                NATIVE_LOGGER.error(message);
                break;
            default:
                NATIVE_LOGGER.error(message);
        }
    }

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
    public void close() throws Exception {
        if (ptr != 0) {
            for (AutoCloseable f : dependedResources) {
                f.close();
            }
            closeRuntime(ptr);
            ptr = 0;
        }
    }

    public QuickJSContext createContext() {
        QuickJSContext result = new QuickJSContext(this);
        this.dependedResources.add(result);
        return result;
    }

}
