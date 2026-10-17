//! Bindings of JDK classes used by the native library, which are not already provided by the `jni` crate.

use jni::bind_java_type;

bind_java_type! {
    rust_type = pub JBoolean,
    java_type = "java.lang.Boolean",

    methods {
        static fn value_of(value: jboolean) -> JBoolean,
        fn boolean_value() -> jboolean,
    },
}

bind_java_type! {
    rust_type = pub JNumber,
    java_type = "java.lang.Number",

    methods {
        fn int_value() -> jint,
        fn long_value() -> jlong,
        fn double_value() -> jdouble,
    },
}

bind_java_type! {
    rust_type = pub JInteger,
    java_type = "java.lang.Integer",

    type_map = {
        JNumber => "java.lang.Number"
    },

    methods {
        static fn value_of(value: jint) -> JInteger,
        fn int_value() -> jint,
    },

    is_instance_of = {
        number: JNumber,
    },
}

bind_java_type! {
    rust_type = pub JLong,
    java_type = "java.lang.Long",

    type_map = {
        JNumber => "java.lang.Number"
    },

    methods {
        fn long_value() -> jlong,
    },

    is_instance_of = {
        number: JNumber,
    },
}

bind_java_type! {
    rust_type = pub JFloat,
    java_type = "java.lang.Float",

    type_map = {
        JNumber => "java.lang.Number"
    },

    methods {
        fn double_value() -> jdouble,
    },

    is_instance_of = {
        number: JNumber,
    },
}

bind_java_type! {
    rust_type = pub JDouble,
    java_type = "java.lang.Double",

    type_map = {
        JNumber => "java.lang.Number"
    },

    methods {
        static fn value_of(value: jdouble) -> JDouble,
        fn double_value() -> jdouble,
    },

    is_instance_of = {
        number: JNumber,
    },
}

bind_java_type! {
    rust_type = pub JBigInteger,
    java_type = "java.math.BigInteger",

    type_map = {
        JNumber => "java.lang.Number"
    },

    methods {
        fn long_value() -> jlong,
    },

    is_instance_of = {
        number: JNumber,
    },
}

bind_java_type! {
    rust_type = pub JBigDecimal,
    java_type = "java.math.BigDecimal",

    type_map = {
        JNumber => "java.lang.Number"
    },

    methods {
        fn to_java_string {
            name = "toString",
            sig = () -> JString,
        },
    },

    is_instance_of = {
        number: JNumber,
    },
}

bind_java_type! {
    rust_type = pub JIterable,
    java_type = "java.lang.Iterable",

    methods {
        fn iterator() -> JIterator,
    },
}

bind_java_type! {
    rust_type = pub JHashSet,
    java_type = "java.util.HashSet",

    constructors {
        fn new(),
    },

    methods {
        fn add(element: JObject) -> jboolean,
    },

    is_instance_of = {
        set: JSet,
    },
}

bind_java_type! {
    rust_type = pub JFunction,
    java_type = "java.util.function.Function",

    methods {
        fn apply(t: JObject) -> JObject,
    },
}

bind_java_type! {
    rust_type = pub JBiFunction,
    java_type = "java.util.function.BiFunction",

    methods {
        fn apply(t: JObject, u: JObject) -> JObject,
    },
}

bind_java_type! {
    rust_type = pub JConsumer,
    java_type = "java.util.function.Consumer",

    methods {
        fn accept(t: JObject),
    },
}

bind_java_type! {
    rust_type = pub JBiConsumer,
    java_type = "java.util.function.BiConsumer",

    methods {
        fn accept(t: JObject, u: JObject),
    },
}

bind_java_type! {
    rust_type = pub JSupplier,
    java_type = "java.util.function.Supplier",

    methods {
        fn get() -> JObject,
    },
}
