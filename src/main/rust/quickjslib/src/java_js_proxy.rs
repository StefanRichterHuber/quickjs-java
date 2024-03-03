use std::rc::Rc;

use jni::errors;
use jni::objects::{JThrowable, JValueGen};
use jni::{
    objects::{JObject, JString},
    JNIEnv,
};
use log::info;
use rquickjs::{BigInt, Exception, Function, IntoJs, Value};

use crate::foreign_function::{function_to_ptr, ptr_to_function};
use crate::js_java_proxy::JSJavaProxy;
use crate::runtime;

/// This the intermediate value when converting a Java to a JS value.
pub enum ProxiedJavaValue {
    THROWABLE(String),
    NULL,
    STRING(String),
    DOUBLE(f64),
    INT(i32),
    BOOL(bool),
    BIGDECIMAL(String),
    BIGINTEGER(i64),
    FUNCTION(Box<dyn Fn(Value<'_>) -> ProxiedJavaValue>),
    BIFUNCTION(Box<dyn Fn(Value<'_>, Value<'_>) -> ProxiedJavaValue>),
    SUPPLIER(Box<dyn Fn() -> ProxiedJavaValue>),
    MAP(Vec<(String, ProxiedJavaValue)>),
    JSFUNCTION(i64),
    ARRAY(Vec<ProxiedJavaValue>),
}

impl ProxiedJavaValue {
    /// Converts a Java value to a ProxiedJavaValue. This is achieved by checking the plain Java Object with `instance of` checks for its real type, then extract all the values to a ProxiedJavaValue.
    /// This involves a lot of call backs into the JVM and has to be optimized, especially for Iterable and Map objects.
    pub fn from_object<'vm>(env: &mut JNIEnv<'vm>, obj: JObject<'vm>) -> Self {
        info!("Calling ProxiedJavaValue::from_object");
        if obj.is_null() {
            runtime::log(runtime::LogLevel::TRACE, "Map Java null to JS null");
            return ProxiedJavaValue::NULL;
        }

        let double_class = env
            .find_class("java/lang/Double")
            .expect("Failed to load the target class");
        let float_class = env
            .find_class("java/lang/Float")
            .expect("Failed to load the target class");
        let int_class = env
            .find_class("java/lang/Integer")
            .expect("Failed to load the target class");
        let long_class = env
            .find_class("java/lang/Long")
            .expect("Failed to load the target class");
        let string_class = env
            .find_class("java/lang/String")
            .expect("Failed to load the target class");
        let bool_class = env
            .find_class("java/lang/Boolean")
            .expect("Failed to load the target class");
        let bigdecimal_class = env
            .find_class("java/math/BigDecimal")
            .expect("Failed to load the target class");
        let biginteger_class = env
            .find_class("java/math/BigInteger")
            .expect("Failed to load the target class");
        let function_class = env
            .find_class("java/util/function/Function")
            .expect("Failed to load the target class");
        let supplier_class = env
            .find_class("java/util/function/Supplier")
            .expect("Failed to load the target class");
        let bifunction_class = env
            .find_class("java/util/function/BiFunction")
            .expect("Failed to load the target class");
        let consumer_class = env
            .find_class("java/util/function/Consumer")
            .expect("Failed to load the target class");
        let map_class = env
            .find_class("java/util/Map")
            .expect("Failed to load the target class");
        let quickjs_function_class = env
            .find_class("com/github/stefanrichterhuber/quickjs/QuickJSFunction")
            .expect("Failed to load the target class");
        let iterable_class = env
            .find_class("java/lang/Iterable")
            .expect("Failed to load the target class");

        if env.is_instance_of(&obj, iterable_class).unwrap() {
            runtime::log(
                runtime::LogLevel::TRACE,
                "Create JS array from Java java.lang.Iterable",
            );

            // Result of the operation -> a list of values
            let mut items: Vec<ProxiedJavaValue> = vec![];

            // Create an iterator over all results
            let iterator_result: errors::Result<JValueGen<JObject<'_>>> =
                env.call_method(&obj, "iterator", "()Ljava/util/Iterator;", &[]);
            let iterator = iterator_result.unwrap().l().unwrap();

            loop {
                // Check if there is another item
                let has_next_result = env.call_method(&iterator, "hasNext", "()Z", &[]);
                let has_next = has_next_result.unwrap().z().unwrap();
                if !has_next {
                    break;
                }

                // Map next item
                let next_result = env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[]);
                let next = next_result.unwrap().l().unwrap();
                let value = ProxiedJavaValue::from_object(env, next);

                // Push result to map
                items.push(value);
            }

            return ProxiedJavaValue::ARRAY(items);
        } else if env.is_instance_of(&obj, quickjs_function_class).unwrap() {
            runtime::log(
                runtime::LogLevel::TRACE,
                "Unwrap Java QuickJSFunction to JS function",
            );

            let ptr_result = env.get_field(&obj, "ptr", "J");
            let ptr = ptr_result.unwrap().j().unwrap();
            ProxiedJavaValue::JSFUNCTION(ptr)
        } else if env.is_instance_of(&obj, map_class).unwrap() {
            runtime::log(
                runtime::LogLevel::TRACE,
                "Copy Java Map<Object, Object> to JS object",
            );

            // Result of the operation -> a list of key-value pairs
            let mut items: Vec<(String, ProxiedJavaValue)> = vec![];

            // Iterate over all items

            // Fetch the entry set
            let entry_set_result: errors::Result<JValueGen<JObject<'_>>> =
                env.call_method(obj, "entrySet", "()Ljava/util/Set;", &[]);

            let entry_set = entry_set_result.unwrap().l().unwrap();

            // Create an iterator over all results
            let iterator_result: errors::Result<JValueGen<JObject<'_>>> =
                env.call_method(entry_set, "iterator", "()Ljava/util/Iterator;", &[]);
            let iterator = iterator_result.unwrap().l().unwrap();

            // FIXME cache method ids for better performance
            loop {
                // Check if there is another item
                let has_next_result = env.call_method(&iterator, "hasNext", "()Z", &[]);
                let has_next = has_next_result.unwrap().z().unwrap();
                if !has_next {
                    break;
                }

                // Map next item
                let next_result = env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[]);
                let next = next_result.unwrap().l().unwrap();

                // Get key
                let get_key_result = env.call_method(&next, "getKey", "()Ljava/lang/Object;", &[]);
                let key = get_key_result.unwrap().l().unwrap().into();
                let key: String = env.get_string(&key).unwrap().into();

                // Get value from entry
                let get_value_result =
                    env.call_method(&next, "getValue", "()Ljava/lang/Object;", &[]);
                let value = get_value_result.unwrap().l().unwrap();
                let value = ProxiedJavaValue::from_object(env, value);

                // Push result to map
                items.push((key, value));
            }

            ProxiedJavaValue::MAP(items)
        } else if env.is_instance_of(&obj, consumer_class).unwrap() {
            let target = Rc::new(env.new_global_ref(obj).unwrap());
            // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
            let vm = env.get_java_vm().unwrap();

            let f = move |v1: Value| {
                let mut env = vm.get_env().unwrap();

                let p1 = JSJavaProxy::new(v1).into_jobject(&mut env).unwrap();
                let _call_result: errors::Result<JValueGen<JObject<'_>>> = env.call_method(
                    target.as_ref(),
                    "accept",
                    "(Ljava/lang/Object;)V",
                    &[jni::objects::JValueGen::Object(&p1)],
                );

                let result = if env.exception_check().unwrap() {
                    let exception = env.exception_occurred().unwrap();
                    ProxiedJavaValue::from_throwable(&mut env, exception)
                } else {
                    ProxiedJavaValue::from_null()
                };

                result
            };
            runtime::log(
                runtime::LogLevel::TRACE,
                "Create JS function from Java Consumer<Object>",
            );
            ProxiedJavaValue::FUNCTION(Box::new(f))
        } else if env.is_instance_of(&obj, bifunction_class).unwrap() {
            let target = Rc::new(env.new_global_ref(obj).unwrap());
            // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
            let vm = env.get_java_vm().unwrap();

            let f = move |v1: Value, v2: Value| {
                let mut env = vm.get_env().unwrap();

                let p1 = JSJavaProxy::new(v1).into_jobject(&mut env).unwrap();
                let p2 = JSJavaProxy::new(v2).into_jobject(&mut env).unwrap();
                let call_result: errors::Result<JValueGen<JObject<'_>>> = env.call_method(
                    target.as_ref(),
                    "apply",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    &[
                        jni::objects::JValueGen::Object(&p1),
                        jni::objects::JValueGen::Object(&p2),
                    ],
                );

                let result = if env.exception_check().unwrap() {
                    let exception = env.exception_occurred().unwrap();
                    ProxiedJavaValue::from_throwable(&mut env, exception)
                } else {
                    let result = call_result.unwrap().l().unwrap();
                    ProxiedJavaValue::from_object(&mut env, result)
                };

                result
            };
            runtime::log(
                runtime::LogLevel::TRACE,
                "Create JS function from Java BiFunction<Object, Object, Object>",
            );
            ProxiedJavaValue::BIFUNCTION(Box::new(f))
        } else if env.is_instance_of(&obj, supplier_class).unwrap() {
            let target = Rc::new(env.new_global_ref(obj).unwrap());
            // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
            let vm = env.get_java_vm().unwrap();

            let f = move || {
                let mut env = vm.get_env().unwrap();

                let call_result: errors::Result<JValueGen<JObject<'_>>> =
                    env.call_method(target.as_ref(), "get", "()Ljava/lang/Object;", &[]);

                let result = if env.exception_check().unwrap() {
                    let exception = env.exception_occurred().unwrap();
                    ProxiedJavaValue::from_throwable(&mut env, exception)
                } else {
                    let result = call_result.unwrap().l().unwrap();
                    ProxiedJavaValue::from_object(&mut env, result)
                };

                result
            };
            runtime::log(
                runtime::LogLevel::TRACE,
                "Create JS function from Java Supplier<Object>",
            );
            ProxiedJavaValue::SUPPLIER(Box::new(f))
        } else if env.is_instance_of(&obj, function_class).unwrap() {
            let target = Rc::new(env.new_global_ref(obj).unwrap());
            // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
            let vm = env.get_java_vm().unwrap();

            let f = move |msg: Value| {
                let mut env = vm.get_env().unwrap();

                let param = JSJavaProxy::new(msg).into_jobject(&mut env).unwrap();
                let call_result: errors::Result<JValueGen<JObject<'_>>> = env.call_method(
                    target.as_ref(),
                    "apply",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                    &[jni::objects::JValueGen::Object(&param)],
                );

                let result = if env.exception_check().unwrap() {
                    let exception = env.exception_occurred().unwrap();
                    ProxiedJavaValue::from_throwable(&mut env, exception)
                } else {
                    let result = call_result.unwrap().l().unwrap();
                    ProxiedJavaValue::from_object(&mut env, result)
                };

                result
            };
            runtime::log(
                runtime::LogLevel::TRACE,
                "Create JS function from Java Function<Object, Object>",
            );
            ProxiedJavaValue::FUNCTION(Box::new(f))
        } else if env.is_instance_of(&obj, biginteger_class).unwrap() {
            // Convert big integer to string -> later on bag to JS big integer
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            runtime::log(
                runtime::LogLevel::TRACE,
                "Map Java BigInteger to JS BigInteger",
            );
            ProxiedJavaValue::BIGINTEGER(value)
        } else if env.is_instance_of(&obj, bigdecimal_class).unwrap() {
            // Convert big decimal to string -> later on bag to JS big decimal
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            runtime::log(
                runtime::LogLevel::TRACE,
                "Map Java BigDecimal to JS BigDecimal",
            );
            ProxiedJavaValue::BIGDECIMAL(plain)
        } else if env.is_instance_of(&obj, double_class).unwrap()
            || env.is_instance_of(&obj, float_class).unwrap()
        {
            let raw_value = env.call_method(&obj, "doubleValue", "()D", &[]);
            let value = raw_value.unwrap().d().unwrap();
            runtime::log(
                runtime::LogLevel::TRACE,
                "Map Java Double / Float to JS Double",
            );
            ProxiedJavaValue::DOUBLE(value)
        } else if env.is_instance_of(&obj, string_class).unwrap() {
            let str: JString = obj.into();
            let plain: String = env.get_string(&str).unwrap().into();
            runtime::log(runtime::LogLevel::TRACE, "Map Java String to JS String");
            ProxiedJavaValue::STRING(plain)
        } else if env.is_instance_of(&obj, long_class).unwrap() {
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            runtime::log(runtime::LogLevel::TRACE, "Map Java Long to JS BigInteger");
            ProxiedJavaValue::BIGINTEGER(value)
        } else if env.is_instance_of(&obj, int_class).unwrap() {
            let raw_value = env.call_method(&obj, "intValue", "()I", &[]);
            let value = raw_value.unwrap().i().unwrap();
            runtime::log(runtime::LogLevel::TRACE, "Map Java Integer to JS int");
            ProxiedJavaValue::INT(value)
        } else if env.is_instance_of(&obj, bool_class).unwrap() {
            let raw_value = env.call_method(&obj, "booleanValue", "()Z", &[]);
            let value = raw_value.unwrap().z().unwrap();
            runtime::log(runtime::LogLevel::TRACE, "Map Java Boolean to JS bool");
            ProxiedJavaValue::BOOL(value)
        } else {
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            runtime::log(
                runtime::LogLevel::TRACE,
                "Map unsupported Java type to JS by calling toString()",
            );
            ProxiedJavaValue::STRING(plain)
        }
    }

    /// Creates a ProxiedJavaValue from a Java Throwable
    pub fn from_throwable<'vm>(env: &mut JNIEnv<'vm>, throwable: JThrowable<'vm>) -> Self {
        // @see https://stackoverflow.com/questions/27072459/how-to-get-the-message-from-a-java-exception-caught-in-jni
        // Seems to be necessary, otherwise fetching the message fails
        env.exception_clear().unwrap();
        let message = env.call_method(&throwable, "getMessage", "()Ljava/lang/String;", &[]);
        let str: JString = message.unwrap().l().unwrap().into();
        let error_msg: String = env.get_string(&str).unwrap().into();

        runtime::log(
            runtime::LogLevel::TRACE,
            "Map Java exception to JS exception",
        );

        runtime::log(runtime::LogLevel::WARN, &error_msg);

        ProxiedJavaValue::THROWABLE(error_msg)
    }

    pub fn from_null() -> Self {
        runtime::log(runtime::LogLevel::TRACE, "Map Java null to JS null");
        ProxiedJavaValue::NULL
    }
}

impl<'js> IntoJs<'js> for ProxiedJavaValue {
    /// Converts a ProxiedJavaValue into a JS value within the given JS context.
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let result = match self {
            ProxiedJavaValue::THROWABLE(msg) => {
                let exception = Exception::from_message(ctx.clone(), &msg).unwrap();

                let v = Value::from_exception(exception);
                Err(ctx.throw(v))
            }
            ProxiedJavaValue::NULL => Ok(Value::new_null(ctx.clone())),
            ProxiedJavaValue::STRING(str) => Ok(Value::from_string(
                rquickjs::String::from_str(ctx.clone(), str.as_str()).unwrap(),
            )),
            ProxiedJavaValue::DOUBLE(v) => Ok(Value::new_float(ctx.clone(), v)),
            ProxiedJavaValue::INT(v) => Ok(Value::new_int(ctx.clone(), v)),
            ProxiedJavaValue::BOOL(v) => Ok(Value::new_bool(ctx.clone(), v)),
            ProxiedJavaValue::BIGDECIMAL(str) => {
                // FIXME BigDecimal currently not supported by rquickjs
                let s: rquickjs::Result<Value> = ctx.eval(format!("{}m", str));
                s
            }
            ProxiedJavaValue::BIGINTEGER(v) => {
                // FIXME BigInteger currently not supported by rquickjs
                let bi = BigInt::from_i64(ctx.clone(), v).unwrap();
                let s = Value::from_big_int(bi);
                Ok(s)
            }
            ProxiedJavaValue::FUNCTION(f) => {
                let func = Function::new(ctx.clone(), f).unwrap();
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::SUPPLIER(f) => {
                let func = Function::new(ctx.clone(), f).unwrap();
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::BIFUNCTION(f) => {
                let func = Function::new(ctx.clone(), f).unwrap();
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::MAP(values) => {
                let obj = rquickjs::Object::new(ctx.clone()).unwrap();

                for value in values.into_iter() {
                    obj.set(value.0.as_str(), value.1.into_js(ctx).unwrap())
                        .unwrap();
                }
                let s = Value::from_object(obj);
                Ok(s)
            }
            ProxiedJavaValue::JSFUNCTION(ptr) => {
                let func = ptr_to_function(ptr);

                let f = func.as_raw();

                let s: Value<'_> = unsafe { Value::from_raw(ctx.clone(), f) };

                // Prevents dropping the function
                _ = function_to_ptr(func);
                Ok(s)
            }
            ProxiedJavaValue::ARRAY(mut values) => {
                let obj = rquickjs::Array::new(ctx.clone()).unwrap();

                for i in 0..values.len() {
                    let value = values.remove(0);
                    let value = value.into_js(ctx).unwrap();
                    obj.set(i, value).unwrap();
                }

                Ok(Value::from_array(obj))
            }
        };
        result
    }
}
