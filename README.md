[![Maven CI](https://github.com/StefanRichterHuber/quickjs-java/actions/workflows/maven.yml/badge.svg)](https://github.com/StefanRichterHuber/quickjs-java/actions/workflows/maven.yml)

# QuickJS Java

This is a Java library to use [QuickJS from Fabrice Bellard](https://bellard.org/quickjs/) with Java. It uses a native library build with Rust which uses [rquickjs](https://github.com/DelSkayn/rquickjs) to interface QuickJS and [jni-rs](https://github.com/jni-rs/jni-rs) for Java - Rust interop.

## Why another JavaScript runtime for Java?

There are several (more) mature JavaScript runtimes for Java like

- [Nashorn](https://github.com/openjdk/nashorn)
- [GraalVM JS](https://www.graalvm.org/latest/reference-manual/js/), which also runs independently of GraalVM

All of these deeply integrate with the Java runtime and allow full access of JS scripts into the Java runtime. For some applications this might be a security or stability issue. This runtime, on the other hand, only has a very lean interface between Java and Javascript. Scripts can only access the objects explicitly passed into the runtime and have no other access to the outside world. Furthermore hard limits on time and memory consumption of the scripts can be easily set, to limit the impact of malicious or faulty scripts on your applications. This is especially great to implement some calculation or validation scripts, so very small scripts with a small, very well defined scope. Due to the safe nature of this runtime, you can pass writing this scripts to trusted users without compromising the integrity of the rest of your application.
A main goal of this implementation is to provide a very clean, yet efficient and type-safe interface.

On the other hand, this library requires a native library to be build, which adds some build complexity (requires Rust with cargo).

There are, however, other projects binding QuickJS to the JVM, which might be worth looking at:

- [Quack](https://github.com/koush/quack): "Quack provides Java (Android and desktop) bindings to JavaScript engines."s
- [QuickJS - KT](https://github.com/dokar3/quickjs-kt): "Run your JavaScript code in Kotlin, asynchronously."

## Build

You need Java 21 and Rust with `cargo` and [`cross`](https://github.com/cross-rs/cross) to build this project. So a simple `mvn clean install` is enough to build and test the whole library. Initial build takes some while because for each target platform a Docker image has to be downloaded.

The `exec-maven-plugin` is used to start the cross build of the native library for several different platforms and afterwards the created library files are copied together using `maven-resources-plugin`. With the file `src/main/rust/quickjslib/Cross.toml` cross is configured for different platforms. Especially a recent version `libclang` is necessary to use Rust `bindgen` to generate the platform-specific bindings to QuickJS within `cross`. Therefore one has to add a new execution to both the `exec-maven-plugin` and the `maven-resources-plugin`.

Currently supported platforms for the native library:

- `aarch64-unknown-linux-gnu`: Linux ARM 64-Bit
- `x86_64-unknown-linux-gnu`:  Linux x86 64-Bit
- `armv7-unknown-linux-gnueabihf`: Linux ARM-32-Bit
- `x86_64-pc-windows-gnu`: Windows x86 64-Bit

## How to use

Import library

```xml
<dependency>
    <groupId>com.github.stefanrichterhuber</groupId>
    <artifactId>quickjs-Java</artifactId>
    <version>[current version]</version>
</dependency>
```

And then create a QuickJS runtime and QuickJS context and start using Javascript.

```Java
try (QuickJSRuntime runtime = new QuickJSRuntime(); // A QuickJSRuntime manages resource limits
    QuickJSContext context = runtime.createContext()) { // A QuickJSContext manages an independent namespace
    
    runtime.withScriptRuntimeLimit(1, TimeUnit.SECONDS);

    // Set global variable. For supported types, see table below.
    context.setGlobal("a", "hello");
    // Get global variable. Always returns (boxed) Object or null. Type-check and cast on the java side. See table of supported types below.
    Object a = context.getGlobal("a");
    Object v = context.eval("3 + 4");
    assertEquals(7, (Integer)v);

    // Set both a function from java as well as create a native JS function
    context.setGlobal("f1", (String a) -> "Hello " + a);
    context.eval("function f2(a) { return 'Hello from JS dear ' + a; };");

    // Both functions can be called by using invoke
    String r1 = (String) context.invoke("f1", "World");
    assertEquals("Hello World", r1);

    String r2 = (String) context.invoke("f2", "World");
    assertEquals("Hello from JS dear World", r2);

    assertEquals(7, (Integer)v);

    // Set both a function from java as well as create a native JS function
    context.setGlobal("f1", (String a) -> "Hello " + a);
    context.eval("function f2(a) { return 'Hello from JS dear ' + a; };");

    // Both functions can be called by using invoke
    String r1 = (String) context.invoke("f1", "World");
    assertEquals("Hello World", r1);

    String r2 = (String) context.invoke("f2", "World");
    assertEquals("Hello from JS dear World", r2);
}
```

For further examples look at `com.github.stefanrichterhuber.quickjs.QuickJSContextTest`.

### Supported types

The rust library seamlessly translates all supported Java types to JS types and back. Translation is always a copy operation so changes to an `object` created from a `Map` won't be written back to map, for example. A Java function imported into the JS context will be exported as `com.github.stefanrichterhuber.quickjs.QuickJSFunction`.
All supported Java types can be used as globals, retrieved as globals or used as function parameters or return values and map values.

| Java type                                                   |      JS Type            |  Remark                                                                                                                                                                       |
|-------------------------------------------------------------|:-----------------------:|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `null`                                                      | `null`                  | Both js `null` and `undefined` are mapped to Java `null`.                                                                                                                     |
| `java.lang.Integer`                                         | `int`                   | -                                                                                                                                                                             |
| `java.lang.Double` / `java.lang.Float`                      | `float` ( 64-bit)       | rquickjs only supports 64-bit floats                                                                                                                                          |
| `java.lang.String`                                          | `string`                | -                                                                                                                                                                             |
| `java.lang.Boolean`                                         | `bool`                  | -                                                                                                                                                                             |
| `java.util.Map<String, ?>`                                  | `object`                | Key is expected to be a String, values can be of any of the supported Java types, including another map or functions!                                                         |
| `java.lang.Iterable<?>`                                     | `array`                 | Iterable is copied value by value to JS array. JS arrays are converted to `java.util.ArrayList`. Values can be of any of the supported Java types.                            |
| `java.lang.Object[]`                                        | `array`                 | Array is copied value by value to JS array.  If extracted back from JS, the array will always return as a `java.util.ArrayList`.                                              |
| `java.util.function.Function<?,?>`                          | `function`              | both parameter and return type could be any of the supported Java types                                                                                                       |
| `java.util.function.Supplier<?>`                            | `function`              | return type could be any of the supported Java types                                                                                                                          |
| `java.util.function.BiFunction<?,?,?>`                      | `function`              | both parameters and return type could be any of the supported Java types                                                                                                      |
| `java.util.function.Consumer<?>`                            | `function`              | parameter could be any of the supported Java types                                                                                                                            |
| `java.util.function.BiConsumer<?, ?>`                       | `function`              | parameter could be any of the supported Java types                                                                                                                            |
| `com.github.stefanrichterhuber.quickjs.VariadicFunction<?>` | `function`              | Java function with an `java.lang.Object` array (variardic parameters) as parameter, a generic solution when other functions don't work. Requires manual casts                 |
| `com.github.stefanrichterhuber.quickjs.QuickJSFunction`     | `function`              | if js returns a function, its converted to a QuickJSFunction which can be called from Java or added back to the JS context where it will be transformed back to a function    |
| `java.lang.Exception`                                       | `Exception`             | Java exceptions are mapped to JS exceptions. JS exceptions are mapped to `com.github.stefanrichterhuber.quickjs.QuickJSScriptException`. File and line-number is preserved, full stacktrace, however, is lost |

### Logging

This library uses log4j2 for logging on the java side and the `log` crate on the Rust side. Log messages from the native library are passed into the JVM and logged using log4j2 with the logger name `[QuickJS native library]`.

## TODO / Known issues

- [ ] Add support for BigInteger and BigDecimal. Requires support from rquickjs library.
- [x] Allow the user to stop script running too long
- [x] Fix issues around parsing float values ( e.g. `eval("2.37")` results in an int `2`): This was happening due to a design decision in QuickJS, which makes parsing floats locale dependent <https://github.com/bellard/quickjs/issues/106>. A workaround is provided.
- [x] Support cross-build of native library in maven, so multiple arches are supported out-of-the box.
- [ ] Implement support for [JSR223 Java Scripting API](https://docs.oracle.com/javase/8/docs/technotes/guides/scripting/prog_guide/api.html)
.

## Architecture

The project consists of three layers:

1. A thin Java layer of only a few Java Classes basically only wrapping the native calls into the rust library. Therefore very few dependencies are used. The `jar-jni` jar is used to locate and load the native library and `log4j-api` is used for logging.
2. A Rust library `quickjslib` which uses the `jni` crate to provide a native interface to Java and the `rquickjs` crate to call `QuickJS`. Using the traits `IntoJs` and `FromJs` provided by `rquickjs` all the conversion between QuickJS objects and Java objects happens on the Rust side of the project. To simplify the type conversion and native interfaces only the boxed version of primitive values is supported. The `log` crate is used for logging, with a custom implementation of the `Log` crate provided, which redirects all message to the Java runtime, where these will be logged using `log4j2`.
3. `QuickJS` runtime.

"Classic" Java JNI was preferred over the newer "Foreign Function and Memory API", since the native library is only planned to be used with Java so it could be tailored to its use. This allows more direct Rust - Java interactions like easily calling Java methods on objects or even create new Java objects using their constructor. A "Foreign Function an Memory API" approach would have resulted in a thinner native layer with far higher implementation effort on the Java side for all the type conversion, especially sacrificing the type and lifetime safety the current rust layer provides for the QuickJS runtime.
There are, however, a few unsafe hacks within the native layer, since the lifetime of the QuickJS runtime, context and an exported functions is not managed by the native Rust layer but by the Java runtime (therefore all of them implementing `java.lang.AutoClosable`), which requires conversion of boxed objects to raw pointers and back. A `java.lang.ref.Cleaner` is used in the `QuickJSRuntime` to ensure a cleanup if the runtime gets garbage collected without proper closing the runtime and dependent resources (contexts and functions).

## License

Licensed under MIT License ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>)
