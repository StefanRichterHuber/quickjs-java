package com.github.stefanrichterhuber.quickjs;

import java.util.Objects;
import java.util.function.Function;

/**
 * If the standard functions from java.util.function are not enough, one can use
 * VariadicFunction. You get a Object array as argument and have to ensure
 * correct number and types of arguments yourself. The signature matches calling
 * function via reflection, however, and is therefore a good fit for generic
 * access.
 * 
 * @param R Result type of the function
 */
@FunctionalInterface
public interface VariadicFunction<R> {
    /**
     * Applies this function to the given arguments.
     *
     * @param args the function arguments
     * @return the function result
     */
    public R apply(Object... args);

    /**
     * Returns a composed function that first applies this function to
     * its input, and then applies the {@code after} function to the result.
     * If evaluation of either function throws an exception, it is relayed to
     * the caller of the composed function.
     *
     * @param <V>   the type of output of the {@code after} function, and of the
     *              composed function
     * @param after the function to apply after this function is applied
     * @return a composed function that first applies this function and then
     *         applies the {@code after} function
     * @throws NullPointerException if after is null
     *
     * @see #compose(Function)
     */
    default <V> VariadicFunction<V> andThen(Function<? super R, ? extends V> after) {
        Objects.requireNonNull(after);
        return (Object... t) -> after.apply(apply(t));
    }
}
