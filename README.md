# QuickJS Java

This is a Java library to use [QuickJS from Fabrice Bellard](https://bellard.org/quickjs/) with Java. It uses a native library build with Rust which uses [rquickjs](https://github.com/DelSkayn/rquickjs) to interface QuickJS and [jni-rs](https://github.com/jni-rs/jni-rs) for Java - Rust interop.

## Why another JavaScript runtime for Java?

There are several (more) mature JavaScript runtimes for Java like

- [Nashorn](https://github.com/openjdk/nashorn)
- [GraalVM JS](https://www.graalvm.org/latest/reference-manual/js/), which also runs independently of GraalVM

All of these deeply integrate with the Java runtime and allow full access of JS scripts into the Java runtime. For some applications this might be a security or stability issue. This runtime, on the other hand, only has a very lean interface between Java and Javascript. Scripts can only access the objects explicitly passed into the runtime and have no other access to the outside world. Furthermore hard limits on time and memory consumption of the scripts can be easily set, to limit the impact af malicious or faulty scripts on your applications This is especially great to implement some calculation or validation scripts, so very small scripts with a small, very well defined scope. Due to the safe nature of this runtime, you can pass writing this scripts to trusted users without compromising the integrity of the rest of your application.

On the other hand, this library requires a native library to be build, which adds some build complexity (requires Rust with cargo).

## Build

You need Java 21 and Rust with cargo to build this project. The `rust-maven-plugin` is used to trigger rust build from maven, so a single

```cli
mvn clean install
```

is enough to build and test the whole library.

## How to use

Import library

```xml
<dependency>
    <groupId>com.github.stefanrichterhuber</groupId>
    <artifactId>quickjs-Java</artifactId>
    <version>1.0-SNAPSHOT</version>
</dependency>
```

And then create a QuickJS runtime and QuickJS context and start using Javascript.

```Java
try (QuickJSRuntime runtime = new QuickJSRuntime();
    QuickJSContext context = runtime.createContext()) {

    // Set global variable. For supported types, see table below.
    context.setGlobal("a", "hello");
    // Get global variable. Always returns (boxed) Object or null. Type-check and cast on the java side. See table of supported types below.
    Object a = context.getGlobal("a");
    // Eval script. Always returns a (boxed) Object or null. Type-check and cast on the java side. See table of supported types below.
    Object v = context.eval("3 + 4");
    assertEquals(7, (Integer)v);
}
```

For further examples look at `com.github.stefanrichterhuber.quickjs.QuickJSContextTest`.

### Supported types

The rust library seamlessly translates all supported Java types to JS types and back. Translation is always a copy operation so changes to an `object` created from a `Map` won't be written back to map, for example. A Java function imported into the JS context will be exported as `com.github.stefanrichterhuber.quickjs.QuickJSFunction`.
All supported Java types can be used as globals, retrieved as globals or used as function parameters or return values and map values.

| Java type                                               |      JS Type            |  Remark                                                                                                                                                                       |
|---------------------------------------------------------|:-----------------------:|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `null`                                                  | `null`                  | Both js `null` and undefined are mapped to Java `null`.                                                                                                                       |
| `java.lang.Integer`                                     | `int`                   | -                                                                                                                                                                             |
| `java.lang.Double` / `java.lang.Float`                  | `float` ( 64-bit)       | rquickjs only supports 64-bit floats                                                                                                                                          |
| `java.lang.String`                                      | `string`                | -                                                                                                                                                                             |
| `java.lang.Boolean`                                     | `bool`                  | -                                                                                                                                                                             |
| `java.util.Map<String, ?>`                              | `object`                | Key is expected to be a String, values can be of any of the supported Java types, including another map or functions!                                                         |
| `java.lang.Iterable<?>`                                 | `array`                 | Iterable is copied value by value to JS array. JS arrays are converted to `java.util.ArrayList`. Values can be of any of the supported Java types.                            |
| `java.lang.Object[]`                                    | `array`                 | Array is copied value by value to JS array.  If extracted back from JS, the array will always return as a `java.util.ArrayList`.                                              |
| `java.util.function.Function<?,?>`                      | `function`              | both parameter and return type could be any of the supported Java types                                                                                                       |
| `java.util.function.Supplier<?>`                        | `function`              | return type could be any of the supported Java types                                                                                                                          |
| `java.util.function.BiFunction<?,?,?>`                  | `function`              | both parameters and return type could be any of the supported Java types                                                                                                      |
| `java.util.function.Consumer<?>`                        | `function`              | parameter could be any of the supported Java types                                                                                                                            |
| `com.github.stefanrichterhuber.quickjs.QuickJSFunction` | `function`              | if js returns a function, its converted to a QuickJSFunction which can be called from Java or added back to the JS context where it will be transformed back to a function    |

### Logging

This library uses log4j2 for logging on the java side and the `log` crate on the Rust side. Log messages from the native library are passed into the JVM and logged using log4j2 with the logger name `[QuickJS native library]`.

## TODO / Known issues

- [ ] Add support for BigInteger and BigDecimal. Requires support from rquickjs library.
- [x] Allow the user to stop script running to long
- [ ] Fix issues around float values ( e.g. `eval("2.37")` results in an int `2`)
- [ ] Support cross-build of native library in maven, so multiple arches are supported out-of-the box.
- [ ] Fix issues with forwarding log messages from native to Java runtime at `trace` level. It results in an infinite loop, because JNI also logs at `trace` level.

## Architecture

The project consists of three layers:

1. A thin Java layer of only a few Java Classes basically only wrapping the native calls into the rust library. Therefore very few dependencies are used. The `jar-jni` jar is used to locate and load the native library and `log4j-api` is used for logging.
2. A Rust library `quickjslib` which uses the `jni` crate to provide a native interface to Java and the `rquickjs` crate to call `QuickJS`. Using the traits `IntoJs` and `FromJs` provided by `rquickjs` all the conversion between QuickJS objects and Java objects happens on the Rust side of the project. To simplify the type conversion and native interfaces only the boxed version of primitive values is supported. The `log` crate is used for logging, with a custom implementation of the `Log` crate provided, which redirects all message to the Java runtime, where these will be logged using `log4j2`.
3. `QuickJS` runtime.

"Classic" Java JNI was preferred over the newer "Foreign Function and Memory API", since the native library is only planned to be used with Java so it could be tailored to its use. This allows more direct Rust - Java interactions like easily calling Java methods on objects or even create new Java objects using their constructor. A "Foreign Function an Memory API" approach would have resulted in a thinner native layer with far higher implementation effort on the Java side for all the type conversion, especially sacrificing the type and lifetime safety the current rust layer provides for the QuickJS runtime.
There are, however, a few unsafe hacks within the native layer, since the lifetime of the QuickJS runtime, context and an exported function is not managed by the native Rust layer but by the Java runtime (therefore all of them implementing `java.lang.AutoClosable`), which requires conversion of boxed objects to raw pointers and back.
