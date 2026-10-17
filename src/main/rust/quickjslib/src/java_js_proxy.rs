use jni::errors::Error;
use jni::objects::{Global, JIterator, JMap, JObjectArray, JString, JThrowable, Reference};
use jni::refs::Cast;
use jni::{jni_sig, jni_str, objects::JObject, Env, JavaVM};
use log::{error, trace, warn};
use rquickjs::function::{IntoJsFunc, ParamRequirement};
use rquickjs::{Atom, BigInt, Exception, FromJs, Function, IntoAtom, IntoJs, Value};

use crate::context::JQuickJSContext;
use crate::foreign_function::{JQuickJSFunction, JVariadicFunction};
use crate::handles::{ArrayHandle, FunctionHandle, ObjectHandle};
use crate::java_types::{
    JBiConsumer, JBiFunction, JBigDecimal, JBigInteger, JBoolean, JConsumer, JDouble, JFloat,
    JFunction, JInteger, JIterable, JLong, JSupplier,
};
use crate::js_array::JQuickJSArray;
use crate::js_java_proxy::JSJavaProxy;
use crate::js_object::JQuickJSObject;

/// Global reference to the java QuickJSContext, used by JS functions implemented in Java
type GlobalContext = Global<JQuickJSContext<'static>>;

/// Calls back into the JVM from within a JS function implemented in Java.
///
/// The thread is already attached (the JS function is invoked from within a native method), so this just
/// provides an `Env` for a fresh local reference frame. JNI errors are mapped into a JS exception.
fn call_java<F>(vm: &JavaVM, f: F) -> ProxiedJavaValue
where
    F: FnOnce(&mut Env) -> jni::errors::Result<ProxiedJavaValue>,
{
    vm.attach_current_thread(f)
        .unwrap_or_else(ProxiedJavaValue::from_jni_error)
}

/// Checks if the given object is an instance of `T` and returns it as `T` if possible.
fn cast<'from, 'any, T: Reference>(
    env: &Env<'_>,
    obj: &'from JObject<'any>,
) -> jni::errors::Result<Option<Cast<'from, 'any, T>>> {
    match env.as_cast::<T>(obj) {
        Ok(value) => Ok(Some(value)),
        Err(Error::WrongObjectType) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Converts a (possibly null) java.lang.String to a Rust String. `null` is mapped to an empty string.
fn to_rust_string(env: &Env<'_>, str: &JString<'_>) -> jni::errors::Result<String> {
    if str.is_null() {
        Ok(String::new())
    } else {
        str.try_to_string(env)
    }
}

/// JS Wrapper for io.github.stefanrichterhuber.quickjs.VariadicFunction
pub struct VariadicFunction {
    target: Global<JVariadicFunction<'static>>,
    context: GlobalContext,
    vm: JavaVM,
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
        let result = call_java(&self.vm, |env| {
            // First convert parameters from js arg array to java object array
            // Create java object array of the necessary size
            let args_array = JObjectArray::<JObject>::new(env, params.len(), JObject::null())?;

            // Convert and copy parameters.
            for i in 0..params.len() {
                let value = params.arg(i);
                if let Some(v) = value {
                    let proxied_value = JSJavaProxy::from_js(params.ctx(), v).unwrap();

                    if let Some(java_object) = proxied_value.into_jobject(&self.context, env) {
                        args_array.set_element(env, i, &java_object)?;
                    } else {
                        error!("Error preparing parameters for io.github.stefanrichterhuber.quickjs.VariadicFunction: Could not convert value at index {} to java object. Set to `null`", i);
                    }
                } else {
                    error!("Error preparing parameters for io.github.stefanrichterhuber.quickjs.VariadicFunction: JS value at index {} is none. Set to `null`", i);
                }
            }

            trace!("Calling java function io.github.stefanrichterhuber.quickjs.VariadicFunction");

            // Then finally call function
            let call_result = self.target.apply(env, &args_array);
            ProxiedJavaValue::from_call_result(env, &self.context, call_result)
        });

        result.into_js(params.ctx())
    }
}

/// This the intermediate value when converting a Java to a JS value.
pub enum ProxiedJavaValue {
    Throwable(String, String, String, i32),
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
    Map(Vec<(ProxiedJavaValue, ProxiedJavaValue)>),
    JSFunction(FunctionHandle),
    Array(Vec<ProxiedJavaValue>),
    NativeArray(ArrayHandle),
    NativeObject(ObjectHandle),
}

impl ProxiedJavaValue {
    /// Converts a JNI error, which occurred while calling into Java, into a JS exception
    fn from_jni_error(e: Error) -> Self {
        ProxiedJavaValue::Throwable(
            e.to_string(),
            "java.lang.RuntimeException".to_string(),
            "".to_string(),
            -1,
        )
    }

    /// Converts the result of a java method call into a ProxiedJavaValue. If the call threw an exception, it is converted to a `Throwable`.
    fn from_call_result<'vm>(
        env: &mut Env<'vm>,
        context: &JQuickJSContext<'_>,
        call_result: jni::errors::Result<JObject<'vm>>,
    ) -> jni::errors::Result<Self> {
        match env.exception_occurred() {
            Some(exception) => ProxiedJavaValue::from_throwable(env, exception),
            None => ProxiedJavaValue::from_object(env, context, call_result?),
        }
    }

    /// Converts the result of a java method call without result into a ProxiedJavaValue. If the call threw an exception, it is converted to a `Throwable`.
    fn from_void_call_result(
        env: &mut Env<'_>,
        call_result: jni::errors::Result<()>,
    ) -> jni::errors::Result<Self> {
        match env.exception_occurred() {
            Some(exception) => ProxiedJavaValue::from_throwable(env, exception),
            None => {
                call_result?;
                ProxiedJavaValue::from_null()
            }
        }
    }

    /// Creates a ProxiedJavaValue from a Java Throwable
    pub fn from_throwable<'vm>(
        env: &mut Env<'vm>,
        throwable: JThrowable<'vm>,
    ) -> jni::errors::Result<Self> {
        // @see https://stackoverflow.com/questions/27072459/how-to-get-the-message-from-a-java-exception-caught-in-jni
        // Seems to be necessary, otherwise fetching the message fails
        env.exception_clear();
        let message = throwable.get_message(env)?;
        let error_msg = to_rust_string(env, &message)?;

        // Get type of exception
        // e.getClass().getName();
        let exception_class = env.get_object_class(&throwable)?;
        let class_name = exception_class.get_name(env)?;
        let class_name = to_rust_string(env, &class_name)?;

        // Get line number and file name for a more detailed exception
        // e.getStackTrace()[0].getLineNumber();
        // e.getStackTrace()[0].getFileName();
        let stack_trace = throwable.get_stack_trace(env)?;

        if !stack_trace.is_null() && stack_trace.len(env)? > 0 {
            let first_trace_element = stack_trace.get_element(env, 0)?;
            let line_number = first_trace_element.get_line_number(env)?;
            let file = first_trace_element.get_file_name(env)?;
            let file = to_rust_string(env, &file)?;

            trace!("Map Java exception to JS exception");

            warn!("{} ({}: {})", error_msg, file, line_number);

            Ok(ProxiedJavaValue::Throwable(
                error_msg,
                class_name,
                file,
                line_number,
            ))
        } else {
            trace!("Map Java exception to JS exception");
            warn!("{}", error_msg);
            Ok(ProxiedJavaValue::Throwable(
                error_msg,
                class_name,
                "".to_string(),
                -1,
            ))
        }
    }

    /// Converts a Java null into a JS null
    pub fn from_null() -> jni::errors::Result<Self> {
        trace!("Map Java null to JS null");
        Ok(ProxiedJavaValue::Null)
    }

    /// Converts a Java Object array into a js array
    pub fn from_array<'vm>(
        env: &mut Env<'vm>,
        context: &JQuickJSContext<'_>,
        array: &JObjectArray<'_>,
    ) -> jni::errors::Result<Self> {
        if array.is_null() {
            ProxiedJavaValue::from_null()
        } else {
            trace!("Create JS array from Java java.lang.Object[]");
            // Result of the operation -> a list of values
            let len = array.len(env)?;
            let mut items: Vec<ProxiedJavaValue> = Vec::with_capacity(len);

            for i in 0..len {
                let element = array.get_element(env, i)?;
                let value = ProxiedJavaValue::from_object(env, context, element)?;
                items.push(value);
            }
            Ok(ProxiedJavaValue::Array(items))
        }
    }

    /// Converts a Java Iterable into a js array
    fn from_iterable<'vm>(
        env: &mut Env<'vm>,
        context: &JQuickJSContext<'_>,
        iterable: &JIterable<'_>,
    ) -> jni::errors::Result<Self> {
        // Check if the iterable is actually a QuickJSArray (wrapping a native JS array)
        if let Some(array) = cast::<JQuickJSArray>(env, iterable)? {
            let handle = ArrayHandle::from_raw(array.ptr(env)?);
            trace!("Unwraped Java QuickJSArray to JS array");
            return Ok(ProxiedJavaValue::NativeArray(handle));
        }

        trace!("Create JS array from Java java.lang.Iterable");

        let iterator: JIterator = iterable.iterator(env)?;
        let mut items: Vec<ProxiedJavaValue> = vec![];
        while iterator.has_next(env)? {
            let next = iterator.next(env)?.unwrap_or(JObject::null());
            items.push(ProxiedJavaValue::from_object(env, context, next)?);
        }

        Ok(ProxiedJavaValue::Array(items))
    }

    /// Converts a Java java.util.Map into a js object
    fn from_map<'vm>(
        env: &mut Env<'vm>,
        context: &JQuickJSContext<'_>,
        map: &JMap<'_>,
    ) -> jni::errors::Result<Self> {
        // Check if the map is actually a QuickJSObject (wrapping a native JS object)
        if let Some(object) = cast::<JQuickJSObject>(env, map)? {
            let handle = ObjectHandle::from_raw(object.ptr(env)?);
            trace!("Unwraped Java QuickJSObject to JS object");
            return Ok(ProxiedJavaValue::NativeObject(handle));
        }

        trace!("Copy Java Map<Object, Object> to JS object",);

        let mut items = vec![];
        let mut entries = map.iter(env)?;
        while let Some(entry) = entries.next(env)? {
            let key = entry.key(env)?;
            let key = ProxiedJavaValue::from_object(env, context, key)?;
            let value = entry.value(env)?;
            let value = ProxiedJavaValue::from_object(env, context, value)?;
            items.push((key, value));
        }

        Ok(ProxiedJavaValue::Map(items))
    }

    /// Creates the global references to the context and the vm handle necessary to call java objects from JS
    fn prepare_callback(
        env: &mut Env<'_>,
        context: &JQuickJSContext<'_>,
    ) -> jni::errors::Result<(GlobalContext, JavaVM)> {
        let context = env.new_global_ref(context)?;
        let vm = env.get_java_vm()?;
        Ok((context, vm))
    }

    /// Wraps a io.github.stefanrichterhuber.quickjs.VariadicFunction into a JS function
    fn from_variadic_function(
        env: &mut Env<'_>,
        context: &JQuickJSContext<'_>,
        function: &JVariadicFunction<'_>,
    ) -> jni::errors::Result<Self> {
        let target = env.new_global_ref(function)?;
        let (context, vm) = ProxiedJavaValue::prepare_callback(env, context)?;
        trace!(
            "Create JS function from Java io.github.stefanrichterhuber.quickjs.VariadicFunction"
        );
        Ok(ProxiedJavaValue::VarFunction(VariadicFunction {
            target,
            context,
            vm,
        }))
    }

    /// Wraps a java.util.function.BiConsumer into a JS function
    fn from_biconsumer(
        env: &mut Env<'_>,
        context: &JQuickJSContext<'_>,
        consumer: &JBiConsumer<'_>,
    ) -> jni::errors::Result<Self> {
        let target = env.new_global_ref(consumer)?;
        let (context, vm) = ProxiedJavaValue::prepare_callback(env, context)?;

        let f = move |v1: Value, v2: Value| {
            trace!("Calling java function java.util.function.BiConsumer");

            call_java(&vm, |env| {
                let p1 = JSJavaProxy::new(v1).into_jobject(&context, env);
                let p2 = JSJavaProxy::new(v2).into_jobject(&context, env);
                let call_result = target.accept(
                    env,
                    p1.as_ref().unwrap_or(&JObject::null()),
                    p2.as_ref().unwrap_or(&JObject::null()),
                );
                ProxiedJavaValue::from_void_call_result(env, call_result)
            })
        };
        trace!("Create JS function from Java BiConsumer<Object, Object>",);
        Ok(ProxiedJavaValue::BiFunction(Box::new(f)))
    }

    /// Wraps a java.util.function.Consumer into a JS function
    fn from_consumer(
        env: &mut Env<'_>,
        context: &JQuickJSContext<'_>,
        consumer: &JConsumer<'_>,
    ) -> jni::errors::Result<Self> {
        let target = env.new_global_ref(consumer)?;
        let (context, vm) = ProxiedJavaValue::prepare_callback(env, context)?;

        let f = move |v1: Value| {
            trace!("Calling java function java.util.function.Consumer");

            call_java(&vm, |env| {
                let p1 = JSJavaProxy::new(v1).into_jobject(&context, env);
                let call_result = target.accept(env, p1.as_ref().unwrap_or(&JObject::null()));
                ProxiedJavaValue::from_void_call_result(env, call_result)
            })
        };
        trace!("Create JS function from Java Consumer<Object>",);
        Ok(ProxiedJavaValue::Function(Box::new(f)))
    }

    /// Wraps a java.util.function.Supplier into a JS function
    fn from_supplier(
        env: &mut Env<'_>,
        context: &JQuickJSContext<'_>,
        supplier: &JSupplier<'_>,
    ) -> jni::errors::Result<Self> {
        let target = env.new_global_ref(supplier)?;
        let (context, vm) = ProxiedJavaValue::prepare_callback(env, context)?;

        let f = move || {
            trace!("Calling java function java.util.function.Supplier");

            call_java(&vm, |env| {
                let call_result = target.get(env);
                ProxiedJavaValue::from_call_result(env, &context, call_result)
            })
        };
        trace!("Create JS function from Java Supplier<Object>",);
        Ok(ProxiedJavaValue::Supplier(Box::new(f)))
    }

    /// Wraps a java.util.function.Function into a JS function
    fn from_function(
        env: &mut Env<'_>,
        context: &JQuickJSContext<'_>,
        function: &JFunction<'_>,
    ) -> jni::errors::Result<Self> {
        let target = env.new_global_ref(function)?;
        let (context, vm) = ProxiedJavaValue::prepare_callback(env, context)?;

        let f = move |msg: Value| {
            trace!("Calling java function java.util.function.Function");

            call_java(&vm, |env| {
                let param = JSJavaProxy::new(msg).into_jobject(&context, env);
                let call_result = target.apply(env, param.as_ref().unwrap_or(&JObject::null()));
                ProxiedJavaValue::from_call_result(env, &context, call_result)
            })
        };
        trace!("Create JS function from Java Function<Object, Object>",);
        Ok(ProxiedJavaValue::Function(Box::new(f)))
    }

    /// Wraps a java.util.function.BiFunction into a JS function
    fn from_bifunction(
        env: &mut Env<'_>,
        context: &JQuickJSContext<'_>,
        function: &JBiFunction<'_>,
    ) -> jni::errors::Result<Self> {
        let target = env.new_global_ref(function)?;
        let (context, vm) = ProxiedJavaValue::prepare_callback(env, context)?;

        let f = move |v1: Value, v2: Value| {
            trace!("Calling java function java.util.function.BiFunction");

            call_java(&vm, |env| {
                let p1 = JSJavaProxy::new(v1).into_jobject(&context, env);
                let p2 = JSJavaProxy::new(v2).into_jobject(&context, env);
                let call_result = target.apply(
                    env,
                    p1.as_ref().unwrap_or(&JObject::null()),
                    p2.as_ref().unwrap_or(&JObject::null()),
                );
                ProxiedJavaValue::from_call_result(env, &context, call_result)
            })
        };
        trace!("Create JS function from Java BiFunction<Object, Object, Object>",);
        Ok(ProxiedJavaValue::BiFunction(Box::new(f)))
    }

    /// Converts a Java object to a ProxiedJavaValue. This is achieved by checking the plain Java Object with `instance of` checks for its real type, then extract all the values to a ProxiedJavaValue.
    /// This involves a lot of call backs into the JVM and has to be optimized, especially for Iterable and Map objects.
    pub fn from_object<'vm>(
        env: &mut Env<'vm>,
        context: &JQuickJSContext<'_>,
        obj: JObject<'vm>,
    ) -> jni::errors::Result<Self> {
        if obj.is_null() {
            trace!("Map Java null to JS null");
            return Ok(ProxiedJavaValue::Null);
        }

        // To minimize the number of calls into the JVM, the checks are roughly ordered with probability of being used (so first simple values, then collections, then functions, then not-well-supported values, then fallback)
        if let Some(value) = cast::<JBoolean>(env, &obj)? {
            trace!("Map Java Boolean to JS bool");
            Ok(ProxiedJavaValue::Bool(value.boolean_value(env)?))
        } else if let Some(value) = cast::<JInteger>(env, &obj)? {
            trace!("Map Java Integer to JS int");
            Ok(ProxiedJavaValue::Int(value.int_value(env)?))
        } else if let Some(value) = cast::<JString>(env, &obj)? {
            trace!("Map Java String to JS String");
            Ok(ProxiedJavaValue::String(to_rust_string(env, &value)?))
        } else if let Some(value) = cast::<JDouble>(env, &obj)? {
            trace!("Map Java Double to JS Double",);
            Ok(ProxiedJavaValue::Double(value.double_value(env)?))
        } else if let Some(value) = cast::<JFloat>(env, &obj)? {
            trace!("Map Java Float to JS Double",);
            Ok(ProxiedJavaValue::Double(value.double_value(env)?))
        } else if let Some(iterable) = cast::<JIterable>(env, &obj)? {
            ProxiedJavaValue::from_iterable(env, context, &iterable)
        } else if let Some(map) = cast::<JMap>(env, &obj)? {
            ProxiedJavaValue::from_map(env, context, &map)
        } else if let Some(function) = cast::<JQuickJSFunction>(env, &obj)? {
            // First check for the special case of QuickJSFunction, because it implements both VariadicFunction and Function
            trace!("Unwrap Java QuickJSFunction to JS function",);
            let handle = FunctionHandle::from_raw(function.ptr(env)?);
            Ok(ProxiedJavaValue::JSFunction(handle))
        } else if let Some(function) = cast::<JVariadicFunction>(env, &obj)? {
            // Then check for the more generic case of VariadicFunction because it also implements Function but has an object array as argument
            ProxiedJavaValue::from_variadic_function(env, context, &function)
        } else if let Some(consumer) = cast::<JConsumer>(env, &obj)? {
            ProxiedJavaValue::from_consumer(env, context, &consumer)
        } else if let Some(consumer) = cast::<JBiConsumer>(env, &obj)? {
            ProxiedJavaValue::from_biconsumer(env, context, &consumer)
        } else if let Some(function) = cast::<JBiFunction>(env, &obj)? {
            ProxiedJavaValue::from_bifunction(env, context, &function)
        } else if let Some(supplier) = cast::<JSupplier>(env, &obj)? {
            ProxiedJavaValue::from_supplier(env, context, &supplier)
        } else if let Some(function) = cast::<JFunction>(env, &obj)? {
            ProxiedJavaValue::from_function(env, context, &function)
        } else if let Some(array) = cast::<JObjectArray>(env, &obj)? {
            ProxiedJavaValue::from_array(env, context, &array)
        } else if let Some(value) = cast::<JBigInteger>(env, &obj)? {
            trace!("Map Java BigInteger to JS BigInteger",);
            Ok(ProxiedJavaValue::BigInteger(value.long_value(env)?))
        } else if let Some(value) = cast::<JBigDecimal>(env, &obj)? {
            // Convert big decimal to string -> later on bag to JS big decimal
            trace!("Map Java BigDecimal to JS BigDecimal",);
            let plain = value.to_java_string(env)?;
            Ok(ProxiedJavaValue::BigDecimal(to_rust_string(env, &plain)?))
        } else if let Some(value) = cast::<JLong>(env, &obj)? {
            trace!("Map Java Long to JS BigInteger");
            Ok(ProxiedJavaValue::BigInteger(value.long_value(env)?))
        } else {
            // java.lang.Object cannot be bound -> call toString() dynamically
            let plain = env
                .call_method(
                    &obj,
                    jni_str!("toString"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            let plain = env.cast_local::<JString>(plain)?;
            trace!("Map unsupported Java type to JS by calling toString()",);
            Ok(ProxiedJavaValue::String(to_rust_string(env, &plain)?))
        }
    }
}

impl<'js> IntoAtom<'js> for ProxiedJavaValue {
    /// Converts a ProxiedJavaValue into an Atom within the given JS context, if possible.
    ///
    /// Only `String`, `Int`, `Double` and `Bool` are supported.
    fn into_atom(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<Atom<'js>> {
        match self {
            ProxiedJavaValue::String(value) => Atom::from_str(ctx.clone(), &value),
            ProxiedJavaValue::Int(value) => Atom::from_i32(ctx.clone(), value),
            ProxiedJavaValue::Double(value) => Atom::from_f64(ctx.clone(), value),
            ProxiedJavaValue::Bool(value) => Atom::from_bool(ctx.clone(), value),
            _ => Err(rquickjs::Error::Exception),
        }
    }
}

impl<'js> IntoJs<'js> for ProxiedJavaValue {
    /// Converts a ProxiedJavaValue into a JS value within the given JS context.
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let result = match self {
            ProxiedJavaValue::Throwable(msg, class_name, file, line) => {
                let exception = Exception::from_message(ctx.clone(), &msg)?;
                exception.set("original_java_exception_class", class_name)?;

                exception.set("original_java_exception_line", line)?;
                exception.set("original_java_exception_file", file)?;

                let v = Value::from_exception(exception);
                Err(ctx.throw(v))
            }
            ProxiedJavaValue::Null => Ok(Value::new_null(ctx.clone())),
            ProxiedJavaValue::String(str) => Ok(Value::from_string(rquickjs::String::from_str(
                ctx.clone(),
                str.as_str(),
            )?)),
            ProxiedJavaValue::Double(v) => Ok(Value::new_float(ctx.clone(), v)),
            ProxiedJavaValue::Int(v) => Ok(Value::new_int(ctx.clone(), v)),
            ProxiedJavaValue::Bool(v) => Ok(Value::new_bool(ctx.clone(), v)),
            ProxiedJavaValue::BigDecimal(str) => {
                // FIXME BigDecimal currently not supported by rquickjs
                let s: rquickjs::Result<Value> = ctx.eval(format!("{}m", str));
                s
            }
            ProxiedJavaValue::BigInteger(v) => {
                let bi = BigInt::from_i64(ctx.clone(), v)?;
                let s = Value::from_big_int(bi);
                Ok(s)
            }
            ProxiedJavaValue::Function(f) => {
                let func = Function::new(ctx.clone(), f)?;
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::Supplier(f) => {
                let func = Function::new(ctx.clone(), f)?;
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::BiFunction(f) => {
                let func = Function::new(ctx.clone(), f)?;
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::Map(values) => {
                let obj = rquickjs::Object::new(ctx.clone())?;

                for value in values.into_iter() {
                    obj.set(value.0, value.1)?;
                }
                let s = Value::from_object(obj);
                Ok(s)
            }
            ProxiedJavaValue::JSFunction(handle) => {
                let func = unsafe { handle.as_ref() };

                let f = func.as_raw();

                let s: Value<'_> = unsafe { Value::from_raw(ctx.clone(), f) };
                Ok(s)
            }
            ProxiedJavaValue::Array(mut values) => {
                let obj = rquickjs::Array::new(ctx.clone())?;

                for i in 0..values.len() {
                    let value = values.remove(0);
                    let value = value.into_js(ctx)?;
                    obj.set(i, value)?;
                }

                Ok(Value::from_array(obj))
            }
            ProxiedJavaValue::VarFunction(f) => {
                let func = Function::new::<JObject, VariadicFunction>(ctx.clone(), f)?;
                let s = Value::from_function(func);
                Ok(s)
            }
            ProxiedJavaValue::NativeArray(handle) => {
                let array = unsafe { handle.as_ref() }
                    .clone()
                    .restore(ctx)?;
                Ok(Value::from_array(array))
            }
            ProxiedJavaValue::NativeObject(handle) => {
                let object = unsafe { handle.as_ref() }
                    .clone()
                    .restore(ctx)?;
                Ok(Value::from_object(object))
            }
        };
        result
    }
}

#[cfg(test)]
mod tests {

    use jni::{jni_sig, jni_str, objects::JObject, objects::JValue, InitArgsBuilder, JNIVersion, JavaVM};
    use log::trace;
    use rquickjs::{Context, IntoJs, Runtime};

    use super::ProxiedJavaValue;
    use crate::context::JQuickJSContext;

    struct JavaLogContext {}

    impl log::Log for JavaLogContext {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            println!("{}", metadata.target());
            if metadata.target().starts_with("jni::") {
                return false;
            }
            true
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                println!("{} - {}", record.level(), record.args());
            }
        }

        fn flush(&self) {
            // Nothing to do here
        }
    }

    fn launch_vm() -> JavaVM {
        // Build the VM properties
        let jvm_args = InitArgsBuilder::new()
            // Pass the JNI API version (default is 8)
            .version(JNIVersion::V1_8)
            // You can additionally pass any JVM options (standard, like a system property,
            // or VM-specific).
            // Here we enable some extra JNI checks useful during development
            .option("-Xcheck:jni")
            .build()
            .unwrap();

        let log_context = JavaLogContext {};
        // Create logging
        log::set_boxed_logger(Box::new(log_context))
            .map(|()| log::set_max_level(log::LevelFilter::Trace))
            .unwrap();

        // Create a new VM
        let jvm: JavaVM = JavaVM::new(jvm_args).unwrap();
        jvm
    }

    #[test]

    fn transform_java_double_to_js_float() {
        let vm = launch_vm();

        trace!("Hello world");
        // TODO create object and convert it to JS
        let value: f64 = 3.1232;
        let proxy = vm
            .attach_current_thread(|env| -> jni::errors::Result<ProxiedJavaValue> {
                let result = env
                    .call_static_method(
                        jni_str!("java/lang/Double"),
                        jni_str!("valueOf"),
                        jni_sig!("(D)Ljava/lang/Double;"),
                        &[JValue::Double(value)],
                    )
                    .expect("Failed to create Double object from value");
                let object = result.l().unwrap();
                ProxiedJavaValue::from_object(env, &JQuickJSContext::null(), object)
            })
            .unwrap();

        if let ProxiedJavaValue::Double(v) = proxy {
            assert_eq!(value, v);
        } else {
            panic!("Conversion to double value failed")
        }

        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let v = proxy.into_js(&ctx).unwrap();
            assert!(v.is_float());
        });
    }
}
