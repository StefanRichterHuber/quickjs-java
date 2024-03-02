# QuickJS Java

This is a Java library to use [QuickJS from Fabrice Bellard](https://bellard.org/quickjs/) with Java. It uses a native library build with Rust which uses [rquickjs](https://github.com/DelSkayn/rquickjs) to interface QuickJS and [jni-rs](https://github.com/jni-rs/jni-rs) for Java - Rust interop.

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

    // Set global variable
    context.setGlobal("a", "hello");
    // Get global variable
    Object a = context.getGlobal("a");
    // Eval script
    Object v = context.eval("3 + 4");
    assertEquals(7, v);
```

For further examples look at `com.github.stefanrichterhuber.quickjs.QuickJSContextTest`.
The rust library seamlessly translates all supported Java types to JS types and back. Translations is always a copy operation so changes to an `object` created from a `Map` won't be written back to map. A Java function imported into the JS context will be exported as `com.github.stefanrichterhuber.quickjs.QuickJSFunction`.
All supported Java types can be used as globals, retrieved as globals or used as function parameters or return values.

| Java type                                             |      JS Type          |  Remark                                                                                                                                                                       |
|-------------------------------------------------------|:---------------------:|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| int                                                   | int                   | -                                                                                                                                                                             |
| double / float                                        | float ( 64-bit)       | rquickjs only supports 64-bit floats                                                                                                                                          |
| String                                                | string                | -                                                                                                                                                                             |
| boolean                                               | bool                  | -                                                                                                                                                                             |
| java.util.Map<String, ?>                              | object                | Key is expected to be a String, values can be of any of the supported Java types, including another map or functions!                                                         |
| java.lang.Iterable<?>                                 | array                 | Iterable is copied value by value to JS array. JS arrays are converted to `java.util.List`. Values can be of any of the supported Java types.
| java.util.function.Function<?,?>                      | function              | both parameter and return type could be any of the supported Java types                                                                                                       |
| java.util.function.Supplier<?>                        | function              | return type could be any of the supported Java types                                                                                                                          |
| java.util.function.BiFunction<?,?,?>                  | function              | both parameters and return type could be any of the supported Java types                                                                                                      |
| java.util.function.Consumer<?>                        | function              | parameter could be any of the supported Java types                                                                                                                            |
| com.github.stefanrichterhuber.quickjs.QuickJSFunction | function              | if js returns a function, its converted to a QuickJSFunction which can be called from Java or added back to the JS context where it will be transformed back to a function    |
