use std::rc::Rc;

use jni::errors;
use jni::objects::{JThrowable, JValueGen};
use jni::{
    objects::{JObject, JString},
    JNIEnv,
};
use log::info;
use rquickjs::{BigInt, Function, IntoJs, Value};

use crate::js_java_proxy::JSJavaProxy;

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
}

impl ProxiedJavaValue {
    pub fn from_object<'vm>(env: &mut JNIEnv<'vm>, obj: JObject<'vm>) -> Self {
        info!("Calling ProxiedJavaValue::from_object");
        if obj.is_null() {
            println!("Java value is null");
            return ProxiedJavaValue::NULL;
        }

        // TODO implement different types
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

        if env.is_instance_of(&obj, map_class).unwrap() {
            println!("Java value is a Map<Object, Object>");
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
                    println!("No more items in the map");
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

                println!("Found value with key: {}", key);

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
            println!("Java value is a Consumer<Object>");
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
            println!("Java value is a BiFunction<Object, Object, Object>");
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
            println!("Java value is a Supplier<Object>");
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
            println!("Java value is a Function<Object, Object>");
            ProxiedJavaValue::FUNCTION(Box::new(f))
        } else if env.is_instance_of(&obj, biginteger_class).unwrap() {
            // Convert big integer to string -> later on bag to JS big integer
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            println!("Java value is a BigInteger: {}", value);
            ProxiedJavaValue::BIGINTEGER(value)
        } else if env.is_instance_of(&obj, bigdecimal_class).unwrap() {
            // Convert big decimal to string -> later on bag to JS big decimal
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            println!("Java value is a BigDecimal: {}", plain);
            ProxiedJavaValue::BIGDECIMAL(plain)
        } else if env.is_instance_of(&obj, double_class).unwrap()
            || env.is_instance_of(&obj, float_class).unwrap()
        {
            let raw_value = env.call_method(&obj, "doubleValue", "()D", &[]);
            let value = raw_value.unwrap().d().unwrap();
            println!("Java value is a double or float: {}", value);
            ProxiedJavaValue::DOUBLE(value)
        } else if env.is_instance_of(&obj, string_class).unwrap() {
            let str: JString = obj.into();
            let plain: String = env.get_string(&str).unwrap().into();
            println!("Java value is a string: {}", plain);
            ProxiedJavaValue::STRING(plain)
        } else if env.is_instance_of(&obj, long_class).unwrap() {
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            println!("Java value is an long: {}", value);
            ProxiedJavaValue::BIGINTEGER(value)
        } else if env.is_instance_of(&obj, int_class).unwrap() {
            let raw_value = env.call_method(&obj, "intValue", "()I", &[]);
            let value = raw_value.unwrap().i().unwrap();
            println!("Java value is an int: {}", value);
            ProxiedJavaValue::INT(value)
        } else if env.is_instance_of(&obj, bool_class).unwrap() {
            let raw_value = env.call_method(&obj, "booleanValue", "()Z", &[]);
            let value = raw_value.unwrap().z().unwrap();
            println!("Java value is a boolean: {}", value);
            ProxiedJavaValue::BOOL(value)
        } else {
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            println!("Java value is of unknown type -> toString(): {}", plain);
            ProxiedJavaValue::STRING(plain)
        }
    }

    pub fn from_throwable<'vm>(env: &mut JNIEnv<'vm>, throwable: JThrowable<'vm>) -> Self {
        // @see https://stackoverflow.com/questions/27072459/how-to-get-the-message-from-a-java-exception-caught-in-jni
        // Seems to be necessary, otherwise fetching the message fails
        env.exception_clear().unwrap();
        let message = env.call_method(&throwable, "getMessage", "()Ljava/lang/String;", &[]);
        let str: JString = message.unwrap().l().unwrap().into();
        let error_msg: String = env.get_string(&str).unwrap().into();
        println!("Java value is an exception {}", error_msg);
        ProxiedJavaValue::THROWABLE(error_msg)
    }

    pub fn from_null() -> Self {
        println!("Java value is null");
        ProxiedJavaValue::NULL
    }
}

impl<'js> IntoJs<'js> for ProxiedJavaValue {
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let result = match self {
            ProxiedJavaValue::THROWABLE(_) => Ok(Value::new_null(ctx.clone())),
            ProxiedJavaValue::NULL => Ok(Value::new_null(ctx.clone())),
            ProxiedJavaValue::STRING(str) => Ok(Value::from_string(
                rquickjs::String::from_str(ctx.clone(), str.as_str()).unwrap(),
            )),
            ProxiedJavaValue::DOUBLE(v) => Ok(Value::new_float(ctx.clone(), v)),
            ProxiedJavaValue::INT(v) => Ok(Value::new_int(ctx.clone(), v)),
            ProxiedJavaValue::BOOL(v) => Ok(Value::new_bool(ctx.clone(), v)),
            ProxiedJavaValue::BIGDECIMAL(str) => {
                // TODO fixme
                let s: rquickjs::Result<Value> = ctx.eval(format!("{}m", str));
                s
            }
            ProxiedJavaValue::BIGINTEGER(v) => {
                // TODO fixme
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
        };
        result
    }
}
