package com.github.stefanrichterhuber.quickjs;

import java.lang.ref.Cleaner;
import java.lang.ref.Cleaner.Cleanable;
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
    /**
     * Use the cleaner to ensure Runtime and dependent resources (like Contexts and
     * Functions) are properly closed
     */
    private static final Cleaner CLEANER = Cleaner.create();
    private final Cleanable cleanable;
    private final CleanJob cleanJob;

    private static final Logger LOGGER = LogManager.getLogger();
    private static final Logger NATIVE_LOGGER = LogManager.getLogger("[QuickJS native library]");

    /**
     * This job for Cleaner first closes all dependent resources (e.g.
     * QuickJSContext and QuickJSFunction) and then the QuickJSRuntime itself within
     * the native layer.
     */
    private static class CleanJob implements Runnable {
        private long ptr;
        /**
         * Keep a reference to all contexts created to prevent memory leaks which
         * results in errors when closing the runtime
         */
        final Set<AutoCloseable> dependedResources = new HashSet<>();

        public CleanJob(final long ptr) {
            this.ptr = ptr;
        }

        @Override
        public void run() {
            if (ptr != 0) {
                for (AutoCloseable f : dependedResources) {
                    try {
                        f.close();
                    } catch (Exception e) {
                        LOGGER.error("Failed to close runtime dependent resource", e);
                    }
                }
                closeRuntime(ptr);
                ptr = 0;
            }
        }
    }

    // Loads the native library and initializes logging
    static {
        // Load the native library
        JarJniLoader.loadLib(
                QuickJSRuntime.class,
                // A platform-specific path is automatically suffixed to path below.
                "/com/github/stefanrichterhuber/quickjs/libs",
                // The "lib" prefix and ".so|.dynlib|.dll" suffix are added automatically as
                // needed.
                "javaquickjs");

        // Initialize native logging. This is only possible once, otherwise the rust log
        // library fails
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

    /**
     * Pointer to native runtime
     */
    private long ptr;

    /**
     * Creates a new native runtime
     * 
     * @return Pointer to the native runtime
     */
    private native long createRuntime();

    /**
     * Closes the native runtime
     * 
     * @param ptr Pointer to the native runtime
     */
    private static native void closeRuntime(long ptr);

    /**
     * Initializes the logging for the native library. Only allowed to be called
     * once!
     * 
     * @param level Log level from 0 (off) to 5 (trace)
     */
    private static native void initLogging(int level);

    /**
     * Sets the memory limit for the javascript runtime
     * 
     * @param ptr   Pointer to the native runtime
     * @param limit Memory limit in bytes
     */
    private static native void setMemoryLimit(long ptr, long limit);

    /**
     * Sets the maximum stack size for the javascript runtime
     * 
     * @param ptr  Pointer to the native runtime
     * @param size Limit in bytes
     */
    private static native void setMaxStackSize(long ptr, long size);

    /**
     * Number of milliseconds a script is allowed to run. Defaults to infinite
     * runtime (scriptRuntimeLimit = -1)
     */
    private long scriptRuntimeLimit = -1;

    /**
     * Time in milliseconds when the script was started. This is used to ensure the
     * script meets its runtime limits
     */
    private long scriptStartTime = -1;

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
        this.cleanJob = new CleanJob(ptr);
        this.cleanable = CLEANER.register(this, this.cleanJob);
    }

    /**
     * Returns the native pointer to the QuickJSRuntime
     * 
     * @return Native pointer
     */
    long getRuntimePointer() {
        if (ptr == 0) {
            throw new IllegalStateException("QuickJSRuntime closed");
        }
        return ptr;
    }

    /**
     * This method is called by the native code regularly to check if the execution
     * of JS has to be interrupted.
     * The JS code continues to run as long as this method returns false.
     * Currently this used to implement a timeout for scripts.
     */
    boolean jsInterrupt() {
        if (this.scriptStartTime > 0 && scriptRuntimeLimit > 0) {
            final boolean result = !(System.currentTimeMillis() - scriptStartTime < scriptRuntimeLimit);
            if (result) {
                LOGGER.debug("Script runtime limit of {} ms reached, interrupting script", scriptRuntimeLimit);
            }
            return result;
        }
        return false;

    }

    /**
     * Callback called by QuickJSContext when a script is started
     */
    void scriptStarted() {
        scriptStartTime = System.currentTimeMillis();
        LOGGER.debug("Script started at time {}", scriptStartTime);
    }

    /**
     * Callback called by QuickJSContext when a script is started
     */
    void scriptFinished() {

        LOGGER.debug("Script finished at time {}. Total runtime {} ms", () -> System.currentTimeMillis(),
                () -> System.currentTimeMillis() - scriptStartTime);
        this.scriptStartTime = -1;
    }

    /**
     * Sets the time a script is allowed to run. Negative values allow for infinite
     * runtime
     * 
     * @param limit Limit to set
     * @param unit  Time Unit of the limit
     * @return this QuickJSRuntime instance for method chaining.
     */
    public QuickJSRuntime withScriptRuntimeLimit(long limit, TimeUnit unit) {
        scriptRuntimeLimit = unit.toMillis(limit);
        return this;
    }

    /**
     * Sets the memory limit of javascript execution to the given number of bytes
     * 
     * @param limit Memory limit in bytes
     * @return this QuickJSRuntime instance for method chaining.
     */
    public QuickJSRuntime withMemoryLimit(long limit) {
        setMemoryLimit(getRuntimePointer(), limit);
        return this;
    }

    /**
     * Sets the maximum stack of javascript execution to the given number of bytes
     * 
     * @param limit Stack size limit in bytes
     * @return this QuickJSRuntime instance for method chaining.
     */
    public QuickJSRuntime withMaxStackSize(long size) {
        setMaxStackSize(getRuntimePointer(), size);
        return this;
    }

    @Override
    public void close() throws Exception {
        cleanable.clean();
    }

    /**
     * Creates a new independent QuickJS context. Each context has its own set of
     * globals
     * 
     * @return QuickJSContext
     */
    public QuickJSContext createContext() {
        QuickJSContext result = new QuickJSContext(this);
        this.cleanJob.dependedResources.add(result);
        return result;
    }

    /**
     * QuickJSRuntimes are equal by their native pointer
     */
    @Override
    public boolean equals(Object obj) {
        return obj instanceof QuickJSRuntime && ((QuickJSRuntime) obj).ptr == this.ptr;
    }

    @Override
    public int hashCode() {
        return ptr == 0 ? 0 : Objects.hash(ptr);
    }
}
