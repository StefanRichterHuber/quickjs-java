use std::rc::Rc;

use jni::errors;
use jni::objects::{GlobalRef, JObjectArray, JThrowable, JValueGen};
use jni::{
    objects::{JObject, JString},
    JNIEnv,
};
use log::{debug, error, warn};
use rquickjs::function::{IntoJsFunc, ParamRequirement};
use rquickjs::{BigInt, Exception, FromJs, Function, IntoJs, Value};

use crate::foreign_function::{function_to_ptr, ptr_to_function};
use crate::js_java_proxy::JSJavaProxy;
pub struct VariadicFunction {
    target: Rc<GlobalRef>,
    context: Rc<GlobalRef>,
    vm: jni::JavaVM,
}

/// JS Wrapper for com.github.stefanrichterhuber.quickjs.VariadicFunction
impl VariadicFunction {
    /// Creates a new VariadicFunction with the necessary references to the function object itself, the global java QuickJSContet and the vm object.
    /// * `target` - A java object of type com.github.stefanrichterhuber.quickjs.VariadicFunction
    /// * `context` - A java object of type com.github.stefanrichterhuber.quickjs.QuickJSContext
    /// * `vm` - A java vm object
    fn new(target: Rc<GlobalRef>, context: Rc<GlobalRef>, vm: jni::JavaVM) -> VariadicFunction {
        VariadicFunction {
            target,
            context,
            vm,
        }
    }
}

impl<'js, P> IntoJsFunc<'js, P> for VariadicFunction {
    fn param_requirements() -> rquickjs::function::ParamRequirement {
        // We cannot give any hint on the number of expected parameters
        ParamRequirement::any()
    }

    fn call<'a>(
        &self,
        params: rquickjs::function::Params<'a, 'js>,
    ) -> rquickjs::Result<Value<'js>> {
        let mut env = self.vm.get_env().unwrap();
        // First convert parameters from js arg array to java object array
        // Create java object array of the necessary size
        let args_array = env
            .new_object_array(params.len() as i32, "Ljava/lang/Object;", JObject::null())
            .unwrap();

        // Convert and copy parameters.
        for i in 0..params.len() {
            let value = params.arg(i);
            if let Some(v) = value {
                let proxied_value = JSJavaProxy::from_js(params.ctx(), v).unwrap();

                if let Some(java_object) = proxied_value.into_jobject(&self.context, &mut env) {
                    env.set_object_array_element(&args_array, i as i32, &java_object)
                        .unwrap();
                } else {
                    error!("Error preparing parameters for com.github.stefanrichterhuber.quickjs.VariadicFunction: Could not convert value at index {} to java object. Set to `null`", i);
                }
            } else {
                error!("Error preparing parameters for com.github.stefanrichterhuber.quickjs.VariadicFunction: JS value at index {} is none. Set to `null`", i);
            }
        }

        debug!("Calling java function com.github.stefanrichterhuber.quickjs.VariadicFunction");

        // Then finally call function
        let call_result = env.call_method(
            self.target.as_ref(),
            "apply",
            "([Ljava/lang/Object;)Ljava/lang/Object;",
            &[jni::objects::JValueGen::Object(&args_array)],
        );

        let result = if env.exception_check().unwrap() {
            let exception = env.exception_occurred().unwrap();
            ProxiedJavaValue::from_throwable(&mut env, exception)
        } else {
            let result = call_result.unwrap().l().unwrap();
            ProxiedJavaValue::from_object(&mut env, self.context.as_obj(), result)
        };

        result.into_js(params.ctx())
    }
}

/// This the intermediate value when converting a Java to a JS value.
pub enum ProxiedJavaValue {
    Throwable(String),
    Null,
    String(String),
    Double(f64),
    Int(i32),
    Bool(bool),
    BigDecimal(String),
    BigInteger(i64),
    Function(Box<dyn Fn(Value<'_>) -> ProxiedJavaValue>),
    VarFunction(VariadicFunction),
    BiFunction(Box<dyn Fn(Value<'_>, Value<'_>) -> ProxiedJavaValue>),
    Supplier(Box<dyn Fn() -> ProxiedJavaValue>),
    Map(Vec<(String, ProxiedJavaValue)>),
    JSFunction(i64),
    Array(Vec<ProxiedJavaValue>),
}

impl ProxiedJavaValue {
    fn is_instance_of<'vm>(class: &str, env: &mut JNIEnv<'vm>, obj: &JObject<'vm>) -> bool {
        let class_obj = env.find_class(class);

        match class_obj {
            Ok(class_obj) => env.is_instance_of(obj, class_obj).unwrap_or(false),
            Err(_) => {
                // Failing to find class causes an exception -> clear it
                // env.exception_describe().unwrap();
                env.exception_clear().unwrap();
                error!("Unable to find class {}", &class);
                false
            }
        }
    }

    /**
     * Checks if the given JObject is actually an array
     */
    fn is_array<'vm>(env: &mut JNIEnv<'vm>, obj: &JObject<'vm>) -> bool {
        let class = env.get_object_class(obj).unwrap();
        let is_array = env.call_method(class, "isArray", "()Z", &[]).unwrap();
        is_array.z().unwrap()
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

        ProxiedJavaValue::Throwable(error_msg)
    }

    /// Converts a Java null into a JS null
    pub fn from_null() -> Self {
        debug!("Map Java null to JS null");
        ProxiedJavaValue::Null
    }

    /// Converts a Java Object array into a js array
    pub fn from_array<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        array: JObjectArray<'vm>,
    ) -> Self {
        if array.is_null() {
            ProxiedJavaValue::from_null()
        } else {
            debug!("Create JS array from Java java.lang.Object[]");
            // Result of the operation -> a list of values
            let mut items: Vec<ProxiedJavaValue> = vec![];

            let len = env.get_array_length(&array).unwrap();

            for i in 0..len {
                let element = env.get_object_array_element(&array, i).unwrap();
                let value = ProxiedJavaValue::from_object(env, context, element);
                items.push(value);
            }
            ProxiedJavaValue::Array(items)
        }
    }

    /// Converts a Java Iterable into a js array
    fn from_iterable<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        debug!("Create JS array from Java java.lang.Iterable");

        // Create an iterator over all results
        let iterator_result: errors::Result<JValueGen<JObject<'_>>> =
            env.call_method(&obj, "iterator", "()Ljava/util/Iterator;", &[]);
        let iterator = iterator_result.unwrap().l().unwrap();

        let items = ProxiedJavaValue::iterator_collect(
            env,
            context,
            iterator,
            Box::new(|env, context, value| ProxiedJavaValue::from_object(env, context, value)),
        );

        ProxiedJavaValue::Array(items)
    }

    /// Unwraps a QuickJSFunction back to a javascript function
    fn from_quick_js_function<'vm>(env: &mut JNIEnv<'vm>, obj: JObject<'vm>) -> Self {
        debug!("Unwrap Java QuickJSFunction to JS function",);

        let ptr_result = env.get_field(&obj, "ptr", "J");
        let ptr = ptr_result.unwrap().j().unwrap();
        ProxiedJavaValue::JSFunction(ptr)
    }

    /// Wraps a com.github.stefanrichterhuber.quickjs.VariadicFunction into a JS function
    fn from_variadic_function<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        let target = Rc::new(env.new_global_ref(obj).unwrap());
        let context = Rc::new(env.new_global_ref(context).unwrap());
        // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
        let vm = env.get_java_vm().unwrap();
        debug!(
            "Create JS function from Java com.github.stefanrichterhuber.quickjs.VariadicFunction"
        );
        ProxiedJavaValue::VarFunction(VariadicFunction::new(target, context, vm))
    }

    /// Wraps a java.util.function.BiConsumer into a JS function
    fn from_biconsumer<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        let target = Rc::new(env.new_global_ref(obj).unwrap());
        let context = Rc::new(env.new_global_ref(context).unwrap());
        // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
        let vm = env.get_java_vm().unwrap();

        let f = move |v1: Value, v2: Value| {
            debug!("Calling java function java.util.function.BiConsumer");

            let mut env = vm.get_env().unwrap();

            let p1 = JSJavaProxy::new(v1)
                .into_jobject(&context, &mut env)
                .unwrap();
            let p2 = JSJavaProxy::new(v2)
                .into_jobject(&context, &mut env)
                .unwrap();
            let _call_result = env
                .call_method(
                    target.as_ref(),
                    "accept",
                    "(Ljava/lang/Object;Ljava/lang/Object;)V",
                    &[
                        jni::objects::JValueGen::Object(&p1),
                        jni::objects::JValueGen::Object(&p2),
                    ],
                )
                .unwrap();

            let result = if env.exception_check().unwrap() {
                let exception = env.exception_occurred().unwrap();
                ProxiedJavaValue::from_throwable(&mut env, exception)
            } else {
                ProxiedJavaValue::from_null()
            };

            result
        };
        debug!("Create JS function from Java BiConsumer<Object, Object>",);
        ProxiedJavaValue::BiFunction(Box::new(f))
    }

    /// Wraps a java.util.function.Consumer into a JS function
    fn from_consumer<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        let target = Rc::new(env.new_global_ref(obj).unwrap());
        let context = Rc::new(env.new_global_ref(context).unwrap());
        // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
        let vm = env.get_java_vm().unwrap();

        let f = move |v1: Value| {
            debug!("Calling java function java.util.function.Consumer");
            let mut env = vm.get_env().unwrap();

            let p1 = JSJavaProxy::new(v1)
                .into_jobject(&context, &mut env)
                .unwrap();
            let _call_result = env
                .call_method(
                    target.as_ref(),
                    "accept",
                    "(Ljava/lang/Object;)V",
                    &[jni::objects::JValueGen::Object(&p1)],
                )
                .unwrap();

            let result = if env.exception_check().unwrap() {
                let exception = env.exception_occurred().unwrap();
                ProxiedJavaValue::from_throwable(&mut env, exception)
            } else {
                ProxiedJavaValue::from_null()
            };

            result
        };
        debug!("Create JS function from Java Consumer<Object>",);
        ProxiedJavaValue::Function(Box::new(f))
    }

    /// Wraps a java.util.function.Supplier into a JS function
    fn from_supplier<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        let target = Rc::new(env.new_global_ref(obj).unwrap());
        let context = Rc::new(env.new_global_ref(context).unwrap());
        // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
        let vm = env.get_java_vm().unwrap();

        let f = move || {
            debug!("Calling java function java.util.function.Supplier");
            let mut env = vm.get_env().unwrap();

            let call_result: errors::Result<JValueGen<JObject<'_>>> =
                env.call_method(target.as_ref(), "get", "()Ljava/lang/Object;", &[]);

            let result = if env.exception_check().unwrap() {
                let exception = env.exception_occurred().unwrap();
                ProxiedJavaValue::from_throwable(&mut env, exception)
            } else {
                let result = call_result.unwrap().l().unwrap();
                ProxiedJavaValue::from_object(&mut env, context.as_obj(), result)
            };

            result
        };
        debug!("Create JS function from Java Supplier<Object>",);
        ProxiedJavaValue::Supplier(Box::new(f))
    }

    /// Wraps a java.util.function.Function into a JS function
    fn from_function<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        let target = Rc::new(env.new_global_ref(obj).unwrap());
        let context = Rc::new(env.new_global_ref(context).unwrap());
        // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
        let vm = env.get_java_vm().unwrap();

        let f = move |msg: Value| {
            debug!("Calling java function java.util.function.Function");
            let mut env = vm.get_env().unwrap();

            let param = JSJavaProxy::new(msg)
                .into_jobject(&context, &mut env)
                .unwrap();
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
                ProxiedJavaValue::from_object(&mut env, context.as_ref(), result)
            };

            result
        };
        debug!("Create JS function from Java Function<Object, Object>",);
        ProxiedJavaValue::Function(Box::new(f))
    }

    /// Wraps a java.util.function.BiFunction into a JS function
    fn from_bifunction<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        let target = Rc::new(env.new_global_ref(obj).unwrap());
        let context = Rc::new(env.new_global_ref(context).unwrap());
        // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
        let vm = env.get_java_vm().unwrap();

        let f = move |v1: Value, v2: Value| {
            debug!("Calling java function java.util.function.BiFunction");
            let mut env = vm.get_env().unwrap();

            let p1 = JSJavaProxy::new(v1)
                .into_jobject(&context, &mut env)
                .unwrap();
            let p2 = JSJavaProxy::new(v2)
                .into_jobject(&context, &mut env)
                .unwrap();
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
                ProxiedJavaValue::from_object(&mut env, context.as_ref(), result)
            };

            result
        };
        debug!("Create JS function from Java BiFunction<Object, Object, Object>",);
        ProxiedJavaValue::BiFunction(Box::new(f))
    }

    /// Converts a Map entry into a pair of String and ProxiedJavaValue
    fn from_map_entry<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> (String, Self) {
        // Get key
        let get_key_result = env.call_method(&obj, "getKey", "()Ljava/lang/Object;", &[]);
        let key = get_key_result.unwrap().l().unwrap().into();
        let key: String = env.get_string(&key).unwrap().into();

        // Get value from entry
        let get_value_result = env.call_method(&obj, "getValue", "()Ljava/lang/Object;", &[]);
        let value = get_value_result.unwrap().l().unwrap();
        let value: ProxiedJavaValue = ProxiedJavaValue::from_object(env, context, value);

        (key, value)
    }

    /// Iterates over all items provided by the given Java iterator, applies the for_each function and collects the result into a vector
    fn iterator_collect<'vm, T>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        iterator: JObject<'vm>,
        for_each: Box<dyn Fn(&mut JNIEnv<'vm>, &JObject<'vm>, JObject<'vm>) -> T + 'vm>,
    ) -> Vec<T> {
        // Result of the operation -> a list of key-value pairs
        let mut items: Vec<T> = vec![];
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

            let value = for_each(env, context, next);
            items.push(value);
        }
        items
    }

    /// Converts a Java java.util.Map into a js object
    fn from_map<'vm>(env: &mut JNIEnv<'vm>, context: &JObject<'vm>, obj: JObject<'vm>) -> Self {
        debug!("Copy Java Map<Object, Object> to JS object",);

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
        let items = ProxiedJavaValue::iterator_collect(
            env,
            context,
            iterator,
            Box::new(|env, context, entry| ProxiedJavaValue::from_map_entry(env, context, entry)),
        );

        ProxiedJavaValue::Map(items)
    }

    /// Converts a Java object to a ProxiedJavaValue. This is achieved by checking the plain Java Object with `instance of` checks for its real type, then extract all the values to a ProxiedJavaValue.
    /// This involves a lot of call backs into the JVM and has to be optimized, especially for Iterable and Map objects.
    pub fn from_object<'vm>(
        env: &mut JNIEnv<'vm>,
        context: &JObject<'vm>,
        obj: JObject<'vm>,
    ) -> Self {
        if obj.is_null() {
            debug!("Map Java null to JS null");
            return ProxiedJavaValue::Null;
        }

        // To minimize the number of calls into the JVM, the checks are roughly ordered with probability of being used (so first simple values, then collections, then functions, then not-well-supported values, then fallback)
        if ProxiedJavaValue::is_instance_of("java/lang/Boolean", env, &obj) {
            let raw_value = env.call_method(&obj, "booleanValue", "()Z", &[]);
            let value = raw_value.unwrap().z().unwrap();
            debug!("Map Java Boolean to JS bool");
            ProxiedJavaValue::Bool(value)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Integer", env, &obj) {
            let raw_value = env.call_method(&obj, "intValue", "()I", &[]);
            let value = raw_value.unwrap().i().unwrap();
            debug!("Map Java Integer to JS int");
            ProxiedJavaValue::Int(value)
        } else if ProxiedJavaValue::is_instance_of("java/lang/String", env, &obj) {
            let str: JString = obj.into();
            let plain: String = env.get_string(&str).unwrap().into();
            debug!("Map Java String to JS String");
            ProxiedJavaValue::String(plain)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Double", env, &obj)
            || ProxiedJavaValue::is_instance_of("java/lang/Float", env, &obj)
        {
            let raw_value = env.call_method(&obj, "doubleValue", "()D", &[]);
            let value = raw_value.unwrap().d().unwrap();
            debug!("Map Java Double / Float to JS Double",);
            ProxiedJavaValue::Double(value)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Iterable", env, &obj) {
            ProxiedJavaValue::from_iterable(env, context, obj)
        } else if ProxiedJavaValue::is_instance_of("java/util/Map", env, &obj) {
            ProxiedJavaValue::from_map(env, context, obj)
        } else if ProxiedJavaValue::is_instance_of(
            "com/github/stefanrichterhuber/quickjs/QuickJSFunction",
            env,
            &obj,
        ) {
            // First check for the special case of QuickJSFunction, because it implements both VariadicFunction and Function
            ProxiedJavaValue::from_quick_js_function(env, obj)
        } else if ProxiedJavaValue::is_instance_of(
            "com/github/stefanrichterhuber/quickjs/VariadicFunction",
            env,
            &obj,
        ) {
            // Then check for the more generic case of VariadicFunction because it also implements Function but has an object array as argument
            ProxiedJavaValue::from_variadic_function(env, context, obj)
        } else if ProxiedJavaValue::is_instance_of("java/util/function/Consumer", env, &obj) {
            ProxiedJavaValue::from_consumer(env, context, obj)
        } else if ProxiedJavaValue::is_instance_of("java/util/function/BiConsumer", env, &obj) {
            ProxiedJavaValue::from_biconsumer(env, context, obj)
        } else if ProxiedJavaValue::is_instance_of("java/util/function/BiFunction", env, &obj) {
            ProxiedJavaValue::from_bifunction(env, context, obj)
        } else if ProxiedJavaValue::is_instance_of("java/util/function/Supplier", env, &obj) {
            ProxiedJavaValue::from_supplier(env, context, obj)
        } else if ProxiedJavaValue::is_instance_of("java/util/function/Function", env, &obj) {
            ProxiedJavaValue::from_function(env, context, obj)
        } else if ProxiedJavaValue::is_array(env, &obj) {
            let array = JObjectArray::from(obj);
            ProxiedJavaValue::from_array(env, context, array)
        } else if ProxiedJavaValue::is_instance_of("java/math/BigInteger", env, &obj) {
            // Convert big integer to string -> later on bag to JS big integer
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            debug!("Map Java BigInteger to JS BigInteger",);
            ProxiedJavaValue::BigInteger(value)
        } else if ProxiedJavaValue::is_instance_of("java/math/BigDecimal", env, &obj) {
            // Convert big decimal to string -> later on bag to JS big decimal
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            debug!("Map Java BigDecimal to JS BigDecimal",);
            ProxiedJavaValue::BigDecimal(plain)
        } else if ProxiedJavaValue::is_instance_of("java/lang/Long", env, &obj) {
            let raw_value = env.call_method(&obj, "longValue", "()J", &[]);
            let value = raw_value.unwrap().j().unwrap();
            debug!("Map Java Long to JS BigInteger");
            ProxiedJavaValue::BigInteger(value)
        } else {
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            debug!("Map unsupported Java type to JS by calling toString()",);
            ProxiedJavaValue::String(plain)
        }
    }
}

impl<'js> IntoJs<'js> for ProxiedJavaValue {
    /// Converts a ProxiedJavaValue into a JS value within the given JS context.
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let result = match self {
            ProxiedJavaValue::Throwable(msg) => {
                let exception = Exception::from_message(ctx.clone(), &msg).unwrap();

                let v = Value::from_exception(exception);
                Err(ctx.throw(v))
            }
            ProxiedJavaValue::Null => Ok(Value::new_null(ctx.clone())),
            ProxiedJavaValue::String(str) => Ok(Value::from_string(
                rquickjs::String::from_str(ctx.clone(), str.as_str()).unwrap(),
            )),
            ProxiedJavaValue::Double(v) => Ok(Value::new_float(ctx.clone(), v)),
            ProxiedJavaValue::Int(v) => Ok(Value::new_int(ctx.clone(), v)),
            ProxiedJavaValue::Bool(v) => Ok(Value::new_bool(ctx.clone(), v)),
            ProxiedJavaValue::BigDecimal(str) => {
                // FIXME BigDecimal currently not supported by rquickjs
                let s: rquickjs::Result<Value> = ctx.eval(format!("{}m", str));
                s
            }
            ProxiedJavaValue::BigInteger(v) => {
                // FIXME BigInteger currently not supported by rquickjs
                let bi = BigInt::from_i64(ctx.clone(), v).unwrap();
                let s = Value::from_big_int(bi);
                Ok(s)
            }
            ProxiedJavaValue::Function(f) => {
                let func = Function::new(ctx.clone(), f).unwrap();
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::Supplier(f) => {
                let func = Function::new(ctx.clone(), f).unwrap();
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::BiFunction(f) => {
                let func = Function::new(ctx.clone(), f).unwrap();
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::Map(values) => {
                let obj = rquickjs::Object::new(ctx.clone()).unwrap();

                for value in values.into_iter() {
                    obj.set(value.0.as_str(), value.1.into_js(ctx).unwrap())
                        .unwrap();
                }
                let s = Value::from_object(obj);
                Ok(s)
            }
            ProxiedJavaValue::JSFunction(ptr) => {
                let func = ptr_to_function(ptr);

                let f = func.as_raw();

                let s: Value<'_> = unsafe { Value::from_raw(ctx.clone(), f) };

                // Prevents dropping the function
                _ = function_to_ptr(func);
                Ok(s)
            }
            ProxiedJavaValue::Array(mut values) => {
                let obj = rquickjs::Array::new(ctx.clone()).unwrap();

                for i in 0..values.len() {
                    let value = values.remove(0);
                    let value = value.into_js(ctx).unwrap();
                    obj.set(i, value).unwrap();
                }

                Ok(Value::from_array(obj))
            }
            ProxiedJavaValue::VarFunction(f) => {
                let func = Function::new::<JObject, VariadicFunction>(ctx.clone(), f).unwrap();
                let s = Value::from_function(func);
                Ok(s)
            }
        };
        result
    }
}
