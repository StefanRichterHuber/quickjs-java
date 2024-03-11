package com.github.stefanrichterhuber.quickjs;

import java.lang.ref.Cleaner;
import java.util.HashSet;
import java.util.Objects;
import java.util.Set;
import java.util.concurrent.TimeUnit;

import org.apache.logging.log4j.Level;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

import io.questdb.jar.jni.JarJniLoader;

/**
 * QuickJSRuntime is the root object for managing QuickJS. It manages the
 * resources (both memory and time) allowed to be used for scripts. It is not
 * thread safe!
 */
public class QuickJSRuntime implements AutoCloseable {
    static final Cleaner CLEANER = Cleaner.create();

    private static final Logger LOGGER = LogManager.getLogger();
    private static final Logger NATIVE_LOGGER = LogManager.getLogger("[QuickJS native library]");

    // Loads the native library and initializes logging
    static {
        JarJniLoader.loadLib(
                QuickJSRuntime.class,
                // A platform-specific path is automatically suffixed to path below.
                "/libs",
                // The "lib" prefix and ".so|.dynlib|.dll" suffix are added automatically as
                // needed.
                "quickjslib");

        if (NATIVE_LOGGER.getLevel() == Level.ERROR || LOGGER.getLevel() == Level.FATAL) {
            initLogging(1);
        } else if (NATIVE_LOGGER.getLevel() == Level.WARN) {
            initLogging(2);
        } else if (NATIVE_LOGGER.getLevel() == Level.INFO) {
            initLogging(3);
        } else if (NATIVE_LOGGER.getLevel() == Level.DEBUG) {
            initLogging(4);
        } else if (NATIVE_LOGGER.getLevel() == Level.TRACE) {
            initLogging(5);
        } else if (NATIVE_LOGGER.getLevel() == Level.OFF) {
            initLogging(0);
        } else {
            LOGGER.warn("Unknown log level " + NATIVE_LOGGER.getLevel() + " , using INFO for native library");
            initLogging(3);
        }
    }

    // Pointer to native runtime
    private long ptr;

    private native long createRuntime();

    private static native void closeRuntime(long ptr);

    private static native void initLogging(int level);

    private static native void setMemoryLimit(long ptr, long limit);

    private static native void setMaxStackSize(long ptr, long size);

    /**
     * Number of milliseconds a script is allowed to run
     */
    private long scriptRuntimeLimit = -1;

    /**
     * Time in milliseconds when the script was started. This is used to ensure the
     * script meets its runtime limits
     */
    private long scriptStartTime;

    /**
     * Keep a reference to all contexts created to prevent memory leaks which
     * results in errors when closing the runtime
     */
    private final Set<AutoCloseable> dependedResources = new HashSet<>();

    /**
     * This method is called by the native code to log a message.
     * 
     * @param level   Log level
     * @param message Message to log
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

    /**
     * Creates a new QuickJSRuntime
     */
    public QuickJSRuntime() {
        ptr = createRuntime();
    }

    long getRuntimePointer() {
        if (ptr == 0) {
            throw new IllegalStateException("QuickJSRuntime closed");
        }
        return ptr;
    }

    /**
     * This method is called by the native code regularly to check if the execution
     * of JS has be interrupted.
     * The JS code continues to run as long as this method returns false.
     * Currently this used to implement a timeout for scripts.
     */
    boolean jsInterrupt() {
        if (this.scriptStartTime > 0 && scriptRuntimeLimit > 0) {
            return !(System.currentTimeMillis() - scriptStartTime < scriptRuntimeLimit);
        }
        return false;

    }

    /**
     * Callback called by QuickJSContext when a script is started
     */
    void scriptStarted() {
        if (this.scriptRuntimeLimit > 0) {
            scriptStartTime = System.currentTimeMillis();
        }
    }

    /**
     * Callback called by QuickJSContext when a script is started
     */
    void scriptFinished() {
        this.scriptStartTime = -1;
    }

    /**
     * Sets the time a script is allowed to run. Negative values allow for infinite
     * runtime
     * 
     * @param limit
     * @param unit
     */
    public void setScriptRuntimeLimit(long limit, TimeUnit unit) {
        scriptRuntimeLimit = unit.toMillis(limit);
    }

    /**
     * Sets the memory limit of javascript execution to the given number of bytes
     * 
     * @param limit
     */
    public void setMemoryLimit(long limit) {
        setMemoryLimit(getRuntimePointer(), limit);
    }

    /**
     * Sets the maximum stack of javascript execution to the given number of bytes
     * 
     * @param limit
     */
    public void setMaxStackSize(long size) {
        setMaxStackSize(getRuntimePointer(), size);
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

    /**
     * Creates a new independent QuickJS context. Each context has its own set of
     * globals
     * 
     * @return QuickJSContext
     */
    public QuickJSContext createContext() {
        QuickJSContext result = new QuickJSContext(this);
        this.dependedResources.add(result);
        return result;
    }

    @Override
    public boolean equals(Object obj) {
        return obj instanceof QuickJSRuntime && ((QuickJSRuntime) obj).ptr == this.ptr;
    }

    @Override
    public int hashCode() {
        return ptr == 0 ? 0 : Objects.hash(ptr);
    }
}
