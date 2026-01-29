//! # java-quickjs
//!
//! `javaquickjs` is a Java JNI wrapper for the QuickJS library. Using the `jni` and the `rquickjs` crates.

pub mod context;
pub mod foreign_function;
mod java_js_proxy;
pub mod js_array;
mod js_java_proxy;
pub mod runtime;
mod with_locale;
