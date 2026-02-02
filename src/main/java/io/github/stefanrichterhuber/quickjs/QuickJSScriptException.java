package io.github.stefanrichterhuber.quickjs;

/**
 * This RuntimeException maps exceptions created by the JS runtime. Since we
 * cannot have a proper stacktrace, line number and file name is manually
 * tracked.
 */
public class QuickJSScriptException extends RuntimeException {
    /**
     * Name of the file the exception occurred in.
     */
    private final String fileName;

    /**
     * Line number the exception occurred in.
     */
    private final Integer lineNumber;

    /**
     * JS stack trace
     */
    @SuppressWarnings("unused")
    private final String jsStackTrace;

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
        this.jsStackTrace = null;
    }

    /**
     * Creates a new QuickJSScriptException. This is instantiated by the native
     * runtime, and therefore has no public constructor.
     *
     * @param cause        the cause of the exception
     * @param message      the message of the exception
     * @param fileName     the file name of the script
     * @param jsStackTrace the file name of the script
     */
    QuickJSScriptException(Throwable cause, String message, String fileName, String jsStackTrace) {
        super(message, cause);
        this.fileName = fileName;
        this.lineNumber = parseLineNumberFromStackTrace(jsStackTrace);
        this.jsStackTrace = jsStackTrace;

    }

    /**
     * Parses the line number from the JS stack trace
     * 
     * @param jsStackTrace Stack trace
     * @return Line number found or null fi none present
     */
    private static Integer parseLineNumberFromStackTrace(String jsStackTrace) {
        // Example stack trace" at <eval> (eval_script:3:11)"
        if (jsStackTrace != null && jsStackTrace.isBlank() == false && jsStackTrace.contains(":")) {
            String[] parts = jsStackTrace.split(":");
            if (parts.length >= 2) {
                try {
                    Integer result = Integer.parseInt(parts[1]);
                    return result;
                } catch (NumberFormatException e) {
                    return null;
                }
            }
        }
        return null;
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
        this.jsStackTrace = null;
    }

    /**
     * Since we cannot have a proper stack trace for Exceptions passing through the
     * JS runtime, file name is manually tracked
     * 
     * @return name of the file
     */
    public String getFileName() {
        return fileName;
    }

    /**
     * Since we cannot have a proper stack trace for Exceptions passing through the
     * JS runtime, line number is manually tracked
     * 
     * @return line in the file
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
