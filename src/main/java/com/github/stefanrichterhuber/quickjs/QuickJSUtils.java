package com.github.stefanrichterhuber.quickjs;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.function.BiFunction;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Supplier;

public class QuickJSUtils {
    private QuickJSUtils() {
        // utility class
    }

    /**
     * Check if the type is supported by the QuickJSContext
     * 
     * @param clazz Type to check
     * @return Is supported?
     */
    public static boolean isSupported(Class<?> clazz) {
        if (clazz == null)
            return true;
        if (Void.class.isAssignableFrom(clazz) || Void.TYPE.isAssignableFrom(clazz))
            return true;
        if (Double.class.isAssignableFrom(clazz) || Double.TYPE.isAssignableFrom(clazz))
            return true;
        if (Float.class.isAssignableFrom(clazz) || Float.TYPE.isAssignableFrom(clazz))
            return true;
        if (String.class.isAssignableFrom(clazz))
            return true;
        if (Integer.class.isAssignableFrom(clazz) || Integer.TYPE.isAssignableFrom(clazz))
            return true;
        if (Boolean.class.isAssignableFrom(clazz) || Boolean.TYPE.isAssignableFrom(clazz))
            return true;
        if (Map.class.isAssignableFrom(clazz))
            return true;
        if (Iterable.class.isAssignableFrom(clazz))
            return true;
        if (Consumer.class.isAssignableFrom(clazz))
            return true;
        if (Supplier.class.isAssignableFrom(clazz))
            return true;
        if (Function.class.isAssignableFrom(clazz))
            return true;
        if (BiFunction.class.isAssignableFrom(clazz))
            return true;
        if (QuickJSFunction.class.isAssignableFrom(clazz))
            return true;
        if (VariadicFunction.class.isAssignableFrom(clazz))
            return true;

        return false;
    }

    /**
     * Check if the type of the object is supported by the QuickJSContext
     * 
     * @param obj Object to check
     * @return Is supported?
     */
    public static boolean isSupported(Object obj) {
        if (obj == null)
            return true;
        return isSupported(obj.getClass());
    }

    /**
     * Utility method to create a Map of any object. All public methods, matching
     * the criteria of supported java types, of the
     * object will be mapped to functions.
     * All method parameters and the return type need to be one of the supported
     * types
     */
    @SuppressWarnings("unchecked")
    public static Map<String, Object> createMapOf(Object obj) {
        if (obj == null) {
            return Collections.emptyMap();
        }
        if (obj instanceof Map) {
            return (Map<String, Object>) obj;
        }

        Map<String, Object> result = new HashMap<>();
        @SuppressWarnings("rawtypes")
        Class c = obj.getClass();
        // Add a function for each method
        for (Method m : c.getMethods()) {
            // Check if this is suitable candidate
            // Return type must be compatible with the supported java types
            if (!isSupported(m.getReturnType())) {
                continue;
            }
            // All parameter types must be compatible with the supported java types
            for (var arg : m.getParameterTypes()) {
                if (!isSupported(arg)) {
                    continue;
                }
            }
            final VariadicFunction<Object> f = (args) -> {
                try {
                    return m.invoke(obj, args);
                } catch (IllegalAccessException | InvocationTargetException e) {
                    throw new RuntimeException(e);
                }
            };
            result.put(m.getName(), f);

        }
        return result;
    }
}
