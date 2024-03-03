use std::rc::Rc;

use jni::errors;
use jni::objects::{JThrowable, JValueGen};
use jni::{
    objects::{JObject, JString},
    JNIEnv,
};
use log::{debug, error, info, trace, warn};
use rquickjs::{BigInt, Exception, Function, IntoJs, Value};

use crate::foreign_function::{function_to_ptr, ptr_to_function};
use crate::js_java_proxy::JSJavaProxy;

#[cfg(test)]
mod tests {

    use jni::{InitArgsBuilder, JNIVersion, JavaVM};
    use rquickjs::{Context, IntoJs, Runtime};

    use super::ProxiedJavaValue;

    fn launch_vm<'vm>() -> JavaVM {
        // Build the VM properties
        let jvm_args = InitArgsBuilder::new()
            // Pass the JNI API version (default is 8)
            .version(JNIVersion::V8)
            // You can additionally pass any JVM options (standard, like a system property,
            // or VM-specific).
            // Here we enable some extra JNI checks useful during development
            .option("-Xcheck:jni")
            .build()
            .unwrap();

        // Create a new VM
        let jvm: JavaVM = JavaVM::new(jvm_args).unwrap();
        jvm
    }

    #[test]
    fn transform_java_double_to_js_float() {
        let vm = launch_vm();
        let mut env = vm.attach_current_thread().unwrap();

        // TODO create object and convert it to JS
        let value: f64 = 3.1232;
        let class = env
            .find_class("java/lang/Double")
            .expect("Failed to load the target class");
        let result = env
            .call_static_method(
                class,
                "valueOf",
                "(D)Ljava/lang/Double;",
                &[jni::objects::JValueGen::Double(value)],
            )
            .expect("Failed to create Integer object from value");
        let object = result.l().unwrap();
        let proxy = ProxiedJavaValue::from_object(&mut env, object);

        if let ProxiedJavaValue::DOUBLE(v) = proxy {
            assert_eq!(value, v);
        } else {
            panic!("Conversion to double value failed")
        }

        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let v = proxy.into_js(&ctx).unwrap();
            assert_eq!(true, v.is_float());
        });
    }
}

lazy_static! {
    static ref METHOD_ID_CACHE: std::sync::Mutex<std::collections::HashMap<String, jni::objects::JMethodID>> = {
        let m = std::sync::Mutex::new(std::collections::HashMap::new());
        m
    };
}

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
    fn is_instance_of<'vm>(class: &str, env: &mut JNIEnv<'vm>, obj: &JObject<'vm>) -> bool {
        let class_obj = env.find_class(class);

        match class_obj {
            Ok(class_obj) => {
                let is_instance = env.is_instance_of(obj, class_obj).unwrap();
                // is_instance.unwrap_or(false)
                is_instance
            }
            Err(_) => {
                // Failing to find class causes an exception -> clear it
                // env.exception_describe().unwrap();
                env.exception_clear().unwrap();
                error!("Unable to find class {}", &class);
                false
            }
        }
    }

    /// Converts a Java value to a ProxiedJavaValue. This is achieved by checking the plain Java Object with `instance of` checks for its real type, then extract all the values to a ProxiedJavaValue.
    /// This involves a lot of call backs into the JVM and has to be optimized, especially for Iterable and Map objects.
    pub fn from_object<'vm>(env: &mut JNIEnv<'vm>, obj: JObject<'vm>) -> Self {
        if obj.is_null() {
            debug!("Map Java null to JS null");
            return ProxiedJavaValue::NULL;
        }

        if ProxiedJavaValue::is_instance_of("java/lang/Iterable", env, &obj) {
            debug!("Create JS array from Java java.lang.Iterable",);

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
        } else if ProxiedJavaValue::is_instance_of(
            "com/github/stefanrichterhuber/quickjs/QuickJSFunction",
            env,
            &obj,
        ) {
            debug!("Unwrap Java QuickJSFunction to JS function",);

            let ptr_result = env.get_field(&obj, "ptr", "J");
            let ptr = ptr_result.unwrap().j().unwrap();
            ProxiedJavaValue::JSFUNCTION(ptr)
        } else if ProxiedJavaValue::is_instance_of("java/util/Map", env, &obj) {
            debug!("Copy Java Map<Object, Object> to JS object",);

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
        } else if ProxiedJavaValue::is_instance_of("java/util/function/Consumer", env, &obj) {
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
            debug!("Create JS function from Java Consumer<Object>",);
            ProxiedJavaValue::FUNCTION(Box::new(f))
        } else if ProxiedJavaValue::is_instance_of("java/util/function/BiFunction", env, &obj) {
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
            debug!("Create JS function from Java BiFunction<Object, Object, Object>",);
            ProxiedJavaValue::BIFUNCTION(Box::new(f))
        } else if ProxiedJavaValue::is_instance_of("java/util/function/Supplier", env, &obj) {
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
            debug!("Create JS function from Java Supplier<Object>",);
            ProxiedJavaValue::SUPPLIER(Box::new(f))
        } else if ProxiedJavaValue::is_instance_of("java/util/function/Function", env, &obj) {
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
            debug!("Create JS function from Java Function<Object, Object>",);
            ProxiedJavaValue::FUNCTION(Box::new(f))
        } else if ProxiedJavaValue::is_instance_of("java/math/BigInteger", env, &obj) {
            // Convert big integer to string -> later on bag to JS big integer
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            debug!("Map Java BigInteger to JS BigInteger",);
            ProxiedJavaValue::BIGINTEGER(value)
        } else if ProxiedJavaValue::is_instance_of("java/math/BigDecimal", env, &obj) {
            // Convert big decimal to string -> later on bag to JS big decimal
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            debug!("Map Java BigDecimal to JS BigDecimal",);
            ProxiedJavaValue::BIGDECIMAL(plain)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Double", env, &obj)
            || ProxiedJavaValue::is_instance_of("java/lang/Float", env, &obj)
        {
            let raw_value = env.call_method(&obj, "doubleValue", "()D", &[]);
            let value = raw_value.unwrap().d().unwrap();
            debug!("Map Java Double / Float to JS Double",);
            ProxiedJavaValue::DOUBLE(value)
        } else if ProxiedJavaValue::is_instance_of("java/lang/String", env, &obj) {
            let str: JString = obj.into();
            let plain: String = env.get_string(&str).unwrap().into();
            debug!("Map Java String to JS String");
            ProxiedJavaValue::STRING(plain)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Long", env, &obj) {
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            debug!("Map Java Long to JS BigInteger");
            ProxiedJavaValue::BIGINTEGER(value)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Integer", env, &obj) {
            let raw_value = env.call_method(&obj, "intValue", "()I", &[]);
            let value = raw_value.unwrap().i().unwrap();
            debug!("Map Java Integer to JS int");
            ProxiedJavaValue::INT(value)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Boolean", env, &obj) {
            let raw_value = env.call_method(&obj, "booleanValue", "()Z", &[]);
            let value = raw_value.unwrap().z().unwrap();
            debug!("Map Java Boolean to JS bool");
            ProxiedJavaValue::BOOL(value)
        } else {
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            debug!("Map unsupported Java type to JS by calling toString()",);
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

        debug!("Map Java exception to JS exception",);

        warn!("{}", error_msg);

        ProxiedJavaValue::THROWABLE(error_msg)
    }

    pub fn from_null() -> Self {
        debug!("Map Java null to JS null");
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
