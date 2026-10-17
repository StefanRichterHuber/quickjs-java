use crate::handles::{ContextHandle, RuntimeHandle};
use crate::java_js_proxy::ProxiedJavaValue;
use crate::js_java_proxy::JSJavaProxy;
use crate::{foreign_function, with_locale};
use jni::objects::{JByteBuffer, JClass, JObjectArray, JThrowable, JValue};
use jni::strings::{JNIStr, JNIString};
use jni::{
    bind_java_type, jni_sig, jni_str,
    objects::{JObject, JString},
    sys::jlong,
    Env,
};
use log::trace;
use log::{debug, warn};
use rquickjs::{Context, Ctx, Error, Value};
use std::cell::RefCell;

bind_java_type! {
    rust_type = pub JQuickJSContext,
    java_type = "io.github.stefanrichterhuber.quickjs.QuickJSContext",

    type_map = {
        unsafe RuntimeHandle => long,
        unsafe ContextHandle => long,
    },

    fields {
        ptr: jlong,
    },

    native_methods {
        extern static fn create_context(runtime: RuntimeHandle) -> jlong,
        extern static fn close_context(handle: ContextHandle),
        extern fn set_global(handle: ContextHandle, name: JString, value: JObject),
        extern fn get_global(handle: ContextHandle, name: JString) -> JObject,
        extern fn eval(handle: ContextHandle, script: JString) -> JObject,
        extern fn eval_buffer(handle: ContextHandle, script: JByteBuffer) -> JObject,
        extern fn invoke(handle: ContextHandle, name: JString, args: JObject[]) -> JObject,
    },
}

bind_java_type! {
    rust_type = pub JQuickJSScriptException,
    java_type = "io.github.stefanrichterhuber.quickjs.QuickJSScriptException",

    constructors {
        fn new(message: JString),
        fn new_with_details {
            sig = (cause: JThrowable, message: JString, file_name: JString, js_stack_trace: JString),
        },
    },

    is_instance_of = {
        JThrowable,
    },
}

/// Throws a `QuickJSScriptException` with the given message.
///
/// The `Err(JavaException)` returned after throwing is ignored, the exception stays pending and propagates to Java
/// once the native method returns.
pub(crate) fn throw_script_exception(env: &mut Env<'_>, msg: impl AsRef<str>) {
    let result = JString::from_str(env, msg.as_ref())
        .and_then(|msg| JQuickJSScriptException::new(env, &msg))
        .and_then(|exception| env.throw(JThrowable::from(exception)));
    if let Err(e) = result {
        if !env.exception_check() {
            warn!("Failed to throw QuickJSScriptException: {}", e);
        }
    }
}

/// Throws an exception of the given class with the given message. See `throw_script_exception`.
pub(crate) fn throw_exception(env: &mut Env<'_>, class: &JNIStr, msg: impl AsRef<str>) {
    let _ = env.throw_new(class, JNIString::from(msg.as_ref()));
}

thread_local! {
    static CONTEXT_STACK: RefCell<Vec<(jlong, Ctx<'static>)>> = const { RefCell::new(Vec::new()) };
}

/// Helper function to get a Ctx from a Context, handling re-entrancy.
pub(crate) fn with_context<F, R>(context: &Context, f: F) -> R
where
    F: FnOnce(Ctx) -> R,
{
    let context_ptr = context as *const _ as jlong;

    // Check if context is already in stack (search reverse)
    let existing_ctx = CONTEXT_STACK.with(|stack| {
        stack
            .borrow()
            .iter()
            .rev()
            .find(|(ptr, _)| *ptr == context_ptr)
            .map(|(_, ctx)| ctx.clone())
    });

    if let Some(ctx) = existing_ctx {
        // Transmute 'static Ctx to 'current Ctx
        // This is safe because we know the context is alive (we have &Context) and we are on the same thread.
        // And if we are re-entering, the original scope that created the Ctx is still active on the stack.
        let ctx: Ctx = unsafe { std::mem::transmute(ctx) };
        return f(ctx);
    }

    context.with(|ctx| {
        let static_ctx: Ctx<'static> = unsafe { std::mem::transmute(ctx.clone()) };
        CONTEXT_STACK.with(|stack| stack.borrow_mut().push((context_ptr, static_ctx)));

        // Use a guard to ensure pop happens even if f panics
        struct ContextGuard;
        impl Drop for ContextGuard {
            fn drop(&mut self) {
                CONTEXT_STACK.with(|stack| stack.borrow_mut().pop());
            }
        }
        let _guard = ContextGuard;

        f(ctx)
    })
}

/// Extracts the context from a java QuickJSContext object
pub(crate) fn get_context_from_quickjs_context<'a>(
    env: &mut Env<'_>,
    ctx: &JQuickJSContext<'_>,
) -> jni::errors::Result<&'a Context> {
    let handle = ContextHandle::from_raw(ctx.ptr(env)?);
    Ok(unsafe { handle.as_ref() })
}

// ---------------------- io.github.stefanrichterhuber.quickjs.QuickJSContext
impl JQuickJSContextNativeInterface for JQuickJSContextAPI {
    type Error = jni::errors::Error;

    fn create_context<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        runtime: RuntimeHandle,
    ) -> Result<jlong, Self::Error> {
        debug!("Created new QuickJS context");
        let runtime = unsafe { runtime.as_ref() };
        let context = Context::full(runtime).expect("Failed to create QuickJS context");
        Ok(ContextHandle::new(context).into())
    }

    fn close_context<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ContextHandle,
    ) -> Result<(), Self::Error> {
        debug!("Closed QuickJS context");
        drop(unsafe { handle.into_box() });
        Ok(())
    }

    fn set_global<'local>(
        env: &mut Env<'local>,
        this: JQuickJSContext<'local>,
        handle: ContextHandle,
        name: JString<'local>,
        value: JObject<'local>,
    ) -> Result<(), Self::Error> {
        let context = unsafe { handle.as_ref() };
        let key_string = name.try_to_string(env)?;
        let value = match ProxiedJavaValue::from_object(env, &this, value) {
            Ok(value) => value,
            Err(e) => {
                throw_script_exception(env, e.to_string());
                return Ok(());
            }
        };

        with_context(context, |ctx| {
            let globals = ctx.globals();
            if let Err(e) = globals.set(&key_string, value) {
                handle_exception(e, &ctx, env);
            }
        });
        Ok(())
    }

    fn get_global<'local>(
        env: &mut Env<'local>,
        this: JQuickJSContext<'local>,
        handle: ContextHandle,
        name: JString<'local>,
    ) -> Result<JObject<'local>, Self::Error> {
        let context = unsafe { handle.as_ref() };
        let key_string = name.try_to_string(env)?;

        let result = with_context(context, |ctx| {
            let globals = ctx.globals();
            let s: Result<JSJavaProxy, _> = globals.get(&key_string);

            match s {
                Ok(s) => s.into_jobject(&this, env).unwrap(),
                Err(e) => {
                    handle_exception(e, &ctx, env);
                    JObject::null()
                }
            }
        });

        Ok(result)
    }

    fn eval<'local>(
        env: &mut Env<'local>,
        this: JQuickJSContext<'local>,
        handle: ContextHandle,
        script: JString<'local>,
    ) -> Result<JObject<'local>, Self::Error> {
        let script_string = script.try_to_string(env)?;
        Ok(eval(env, &this, handle, script_string))
    }

    fn eval_buffer<'local>(
        env: &mut Env<'local>,
        this: JQuickJSContext<'local>,
        handle: ContextHandle,
        script: JByteBuffer<'local>,
    ) -> Result<JObject<'local>, Self::Error> {
        let address = env.get_direct_buffer_address(&script)?;

        let result = if address.is_null() {
            warn!("script buffer address is null");
            JObject::null()
        } else {
            let byte_buffer_size = env.get_direct_buffer_capacity(&script)?;
            let script_buf: &[u8] =
                unsafe { core::slice::from_raw_parts(address, byte_buffer_size) };

            eval(env, &this, handle, script_buf)
        };
        Ok(result)
    }

    fn invoke<'local>(
        env: &mut Env<'local>,
        this: JQuickJSContext<'local>,
        handle: ContextHandle,
        name: JString<'local>,
        args: JObjectArray<'local>,
    ) -> Result<JObject<'local>, Self::Error> {
        let context = unsafe { handle.as_ref() };
        let function_name = name.try_to_string(env)?;

        with_context(context, |ctx| {
            let globals = ctx.globals();
            let f: Result<rquickjs::Value, _> = if function_name.contains('.') {
                let parts = function_name.split('.').collect::<Vec<&str>>();

                let mut target = globals;
                let function_name = parts.last().unwrap();
                for part in parts.iter().take(parts.len() - 1) {
                    let s: Result<Value, _> = target.get(*part);
                    target = match s {
                        Ok(s) => {
                            if s.is_object() {
                                s.into_object().unwrap()
                            } else {
                                throw_exception(
                                    env,
                                    jni_str!("java/lang/Exception"),
                                    format!("{} is not an object", part),
                                );
                                return Ok(JObject::null());
                            }
                        }
                        Err(e) => {
                            handle_exception(e, &ctx, env);
                            return Ok(JObject::null());
                        }
                    }
                }
                target.get(*function_name)
            } else {
                globals.get(&function_name)
            };

            // First, try to a global object in the context with the given name
            match f {
                Ok(f) => {
                    // Then check if the global object found is a function. If it is, invoke it with the given arguments. If it is not, throw an exception.
                    if f.is_function() {
                        trace!("Invoking JS function with name {}()", function_name);
                        let func = f.as_function().unwrap();
                        foreign_function::invoke_js_function_with_java_parameters(
                            env, &this, func, &args,
                        )
                    } else {
                        throw_exception(
                            env,
                            jni_str!("java/lang/Exception"),
                            format!("{} is not a function", function_name),
                        );
                        Ok(JObject::null())
                    }
                }
                Err(e) => {
                    handle_exception(e, &ctx, env);
                    Ok(JObject::null())
                }
            }
        })
    }
}

/// Internal common implementation of the script eval
fn eval<'a, S: Into<Vec<u8>>>(
    env: &mut Env<'a>,
    this: &JQuickJSContext<'_>,
    handle: ContextHandle,
    script: S,
) -> JObject<'a> {
    let context = unsafe { handle.as_ref() };
    let locale = with_locale::TemporaryLocale::new_default();

    with_context(context, move |ctx| {
        let s: Result<JSJavaProxy, _> = locale.with(|| ctx.eval(script));

        match s {
            Ok(s) => s.into_jobject(this, env).unwrap(),
            Err(e) => {
                handle_exception(e, &ctx, env);
                JObject::null()
            }
        }
    })
}

/// Handle JS errors. Extracts the message and throws a Java exception..
pub(crate) fn handle_exception(
    e: Error,
    js_context: &rquickjs::Ctx<'_>,
    env: &mut Env<'_>,
) {
    match e {
        Error::Exception => {
            let catch = js_context.catch();
            if let Some(execp) = catch.as_exception() {
                let original_file: Option<String> = execp
                    .get::<_, Value>("original_java_exception_file")
                    .ok()
                    .and_then(|v| v.as_string().and_then(|s| s.to_string().ok()));
                let original_class: Option<String> = execp
                    .get::<_, Value>("original_java_exception_class")
                    .ok()
                    .and_then(|v| v.as_string().and_then(|s| s.to_string().ok()));

                let result = throw_js_exception(
                    env,
                    execp.message(),
                    // Set default filename if none set
                    original_file.unwrap_or_else(|| "<script>".to_string()),
                    execp.stack(),
                    original_class,
                );
                if let Err(e) = result {
                    if !env.exception_check() {
                        throw_script_exception(env, e.to_string());
                    }
                }
            } else if let Some(msg) = catch.as_string() {
                throw_script_exception(env, msg.to_string().unwrap_or_default());
            } else {
                throw_script_exception(env, "Unknown type of JS Error::Exception");
            }
        }
        _ => throw_script_exception(env, e.to_string()),
    }
}

/// Throws a QuickJSScriptException for a JS exception. If the JS exception originates from a Java exception, a new instance
/// of the original exception class is created as cause.
fn throw_js_exception(
    env: &mut Env<'_>,
    message: Option<String>,
    file_name: String,
    stack_trace: Option<String>,
    original_class: Option<String>,
) -> jni::errors::Result<()> {
    let message: JString = match message {
        Some(message) => JString::from_str(env, message)?,
        None => JString::null(),
    };
    let file_name = JString::from_str(env, file_name)?;
    let stack_trace: JString = match stack_trace {
        Some(stack) => JString::from_str(env, stack)?,
        None => JString::null(),
    };

    // First generate the 'cause' parameter for the invocation, if the origin was a java exception
    let cause: JThrowable = match original_class {
        Some(class_name) => {
            // java.lang.Exception -> java/lang/Exception
            let class_name = class_name.replace('.', "/");
            match env.find_class(JNIString::from(&class_name)) {
                Ok(exception_class) => {
                    let cause = env.new_object(
                        exception_class,
                        jni_sig!("(Ljava/lang/String;)V"),
                        &[JValue::Object(&message)],
                    );
                    match cause {
                        Ok(cause) => env.cast_local::<JThrowable>(cause)?,
                        Err(_) => {
                            // e.g. no (String) constructor -> no cause
                            env.exception_clear();
                            JThrowable::null()
                        }
                    }
                }
                Err(_) => {
                    env.exception_clear();
                    warn!("Could not find class '{}'", class_name);
                    JThrowable::null()
                }
            }
        }
        None => {
            debug!("JS exception does not originate from a Java Exception");
            JThrowable::null()
        }
    };

    // Then generate the actual exception
    let exception =
        JQuickJSScriptException::new_with_details(env, &cause, &message, &file_name, &stack_trace)?;
    env.throw(JThrowable::from(exception))
}
