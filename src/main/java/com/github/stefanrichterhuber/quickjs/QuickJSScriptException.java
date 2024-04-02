package com.github.stefanrichterhuber.quickjs;

/**
 * This RuntimeException maps exceptions created by the JS runtime. Since we
 * cannot have a proper stacktrace, line number and file name is manually
 * tracked.
 */
public class QuickJSScriptException extends RuntimeException {
    private final String fileName;
    private final Integer lineNumber;

    /**
     * Creates a new QuickJSScriptException. This is instantiated by the native
     * runtime, and therefore has no public constructor.
     *
     * @param cause      the cause of the exception
     * @param message    the message of the exception
     * @param fileName   the file name of the script
     * @param lineNumber the line number of the script
     */
    QuickJSScriptException(Throwable cause, String message, String fileName, Integer lineNumber) {
        super(message, cause);
        this.fileName = fileName;
        this.lineNumber = lineNumber;
    }

    /**
     * Creates a new QuickJSScriptException. This is instantiated by the native
     * runtime, and therefore has no public constructor.
     *
     * @param message the message of the exception
     */
    QuickJSScriptException(String message) {
        super(message);
        this.fileName = null;
        this.lineNumber = null;
    }

    /**
     * Since we cannot have a proper stack trace for Exceptions passing through the
     * JS runtime, file name is manually tracked
     * 
     * @return
     */
    public String getFileName() {
        return fileName;
    }

    /**
     * Since we cannot have a proper stack trace for Exceptions passing through the
     * JS runtime, line number is manually tracked
     * 
     * @return
     */
    public Integer getLineNumber() {
        return lineNumber;
    }

    @Override
    public String toString() {
        if (fileName != null && lineNumber != null) {
            return String.format("%s (%s:%d)", super.getMessage(), fileName, lineNumber);
        }
        if (lineNumber != null) {
            return String.format("%s (%d)", super.getMessage(), lineNumber);
        }
        if (fileName != null) {
            return String.format("%s (%s)", super.getMessage(), fileName);
        }
        return super.toString();
    }
}
