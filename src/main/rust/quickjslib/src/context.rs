use crate::java_js_proxy::ProxiedJavaValue;
use crate::js_java_proxy::JSJavaProxy;
use crate::runtime::{ptr_to_runtime, runtime_to_ptr};
use crate::{foreign_function, with_locale};
use jni::objects::{JByteBuffer, JObjectArray, JThrowable};
use jni::{
    objects::{JObject, JString},
    sys::jlong,
    JNIEnv,
};
use log::trace;
use log::{debug, warn};
use rquickjs::atom::PredefinedAtom;
use rquickjs::{Context, Error, Value};

// ---------------------- com.github.stefanrichterhuber.quickjs.QuickJSContext
/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.createContext(long ptr)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_createContext<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    runtime_ptr: jlong,
) -> jlong {
    debug!("Created new QuickJS context");
    let runtime = ptr_to_runtime(runtime_ptr);
    let context = Context::full(&runtime).unwrap();

    // Prevents dropping the runtime
    _ = runtime_to_ptr(runtime);

    Box::into_raw(Box::new(context)) as jlong
}

/// Converts a pointer to a context back to a Box<Context>.
fn ptr_to_context(context_ptr: jlong) -> Box<Context> {
    unsafe { Box::from_raw(context_ptr as *mut Context) }
}

fn context_to_ptr(context: Box<Context>) -> jlong {
    Box::into_raw(context) as jlong
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.closeContext(long)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_closeContext<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
) {
    debug!("Closed QuickJS context");
    let context = ptr_to_context(context_ptr);
    drop(context);
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.getGlobal(long, String)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_getGlobal<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    key: JString<'a>,
) -> JObject<'a> {
    let context = ptr_to_context(context_ptr);
    let key_string: String = _env
        .get_string(&key)
        .expect("Couldn't get java string!")
        .into();

    let result = context.with(|ctx| {
        let globals = ctx.globals();
        let s: Result<JSJavaProxy, _> = globals.get(&key_string);

        match s {
            Ok(s) => s.into_jobject(&_obj, &mut _env).unwrap(),
            Err(e) => {
                handle_exception(e, &ctx, &_obj, &mut _env);
                JObject::null()
            }
        }
    });

    // Prevents dropping the context
    _ = context_to_ptr(context);

    result
}

/// Handle JS errors. Extracts the message and throws a Java exception..
pub(crate) fn handle_exception<'vm>(
    e: Error,
    js_context: &rquickjs::Ctx<'_>,
    java_context: &JObject<'vm>,
    env: &mut JNIEnv<'vm>,
) {
    match e {
        Error::Exception => {
            let catch = js_context.catch();
            if let Some(execp) = catch.as_exception() {
                let file_name = JObject::null();


                let stack_trace = if let Some(stack) = execp.stack() {
                    env.new_string(stack).unwrap().into()
                } else {
                    JObject::null()
                };

                // Set default filename if none set
                let file_name = if env.is_same_object(JObject::null(), &file_name).unwrap() {
                    let str = rquickjs::String::from_str(js_context.clone(), "<script>").unwrap();
                    JSJavaProxy::new(Value::from_string(str))
                        .into_jobject(java_context, env)
                        .unwrap()
                } else {
                    file_name
                };

                let java_file_name: Value = execp.get("original_java_exception_file").unwrap();
                let java_file_name = if java_file_name.is_null() || java_file_name.is_undefined() {
                    file_name
                } else {
                    JSJavaProxy::new(java_file_name)
                        .into_jobject(java_context, env)
                        .unwrap_or(file_name)
                };

                let message = execp
                    .get(PredefinedAtom::Message)
                    .map(|v: JSJavaProxy| {
                        v.into_jobject(java_context, env).unwrap_or(JObject::null())
                    })
                    .unwrap_or(JObject::null());

                let original_exception: Result<Value, _> =
                    execp.get("original_java_exception_class");

                // First generate the 'cause' parameter for the invocation, if the origin was a java exception
                let throwable = if let Ok(class_name) = original_exception {
                    if class_name.is_undefined() || class_name.is_null() {
                        JObject::null()
                    } else {
                        // java.lang.Exception -> java/lang/Exception
                        let class_name = class_name.as_string().unwrap().to_string().unwrap();
                        let class_name = class_name.replace('.', "/");

                        if let Ok(exception_class) = env.find_class(&class_name) {
                            let exception_object = env
                                .new_object(
                                    exception_class,
                                    "(Ljava/lang/String;)V",
                                    &[jni::objects::JValueGen::Object(&message)],
                                )
                                .unwrap_or(JObject::null());
                            exception_object
                        } else {
                            env.exception_clear().unwrap();
                            warn!("Could not find class '{}'", class_name);
                            JObject::null()
                        }
                    }
                } else {
                    debug!("JS exception does not originate from a Java Exception");
                    JObject::null()
                };

                // Then generate the actual exception
                let exception_class = env
                    .find_class("com/github/stefanrichterhuber/quickjs/QuickJSScriptException")
                    .unwrap();

                let exception_object:JThrowable = env.new_object(
                    exception_class,
                    "(Ljava/lang/Throwable;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                    &[
                        jni::objects::JValueGen::Object(&throwable),
                        jni::objects::JValueGen::Object(&message),
                        jni::objects::JValueGen::Object(&java_file_name),
                        jni::objects::JValueGen::Object(&stack_trace),
                        ],
                ).unwrap_or(JObject::null()).into();

                env.throw(exception_object).unwrap();
            } else if let Some(msg) = catch.as_string() {
                env.throw_new(
                    "com/github/stefanrichterhuber/quickjs/QuickJSScriptException",
                    msg.to_string().unwrap(),
                )
                .unwrap();
            } else {
                env.throw_new(
                    "com/github/stefanrichterhuber/quickjs/QuickJSScriptException",
                    "Unknown type of JS Error::Exception",
                )
                .unwrap();
            }
        }
        _ => env
            .throw_new(
                "com/github/stefanrichterhuber/quickjs/QuickJSScriptException",
                e.to_string(),
            )
            .unwrap(),
    }
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.setGlobal(long, String, Object)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_setGlobal__JLjava_lang_String_2Ljava_lang_Object_2<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    key: JString<'a>,
    value: JObject<'a>,
) {
    let context = ptr_to_context(context_ptr);
    let key_string: String = _env
        .get_string(&key)
        .expect("Couldn't get java string!")
        .into();
    let value = ProxiedJavaValue::from_object(&mut _env, &_obj, value);

    context.with(|ctx| {
        let globals = ctx.globals();
        let s = globals.set(&key_string, value);

        match s {
            Ok(_) => {}
            Err(e) => {
                handle_exception(e, &ctx, &_obj, &mut _env);
            }
        }
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.eval(long, String)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_eval<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    script: JString<'a>,
) -> JObject<'a> {
    let script_string: String = _env
        .get_string(&script)
        .expect("Couldn't get java string!")
        .into();
    eval(_env, _obj, context_ptr, script_string)
}

/// Internal common implementation of the script eval
pub(crate) fn eval<'a, S: Into<Vec<u8>>>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    script: S,
) -> JObject<'a> {
    let context = ptr_to_context(context_ptr);
    let locale = with_locale::TemporaryLocale::new_default();

    let result = context.with(move |ctx| {
        let s: Result<JSJavaProxy, _> = locale.with(|| ctx.eval(script));

        match s {
            Ok(s) => s.into_jobject(&_obj, &mut env).unwrap(),
            Err(e) => {
                handle_exception(e, &ctx, &_obj, &mut env);
                JObject::null()
            }
        }
    });

    // Prevents dropping the context
    _ = context_to_ptr(context);

    result
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.evalBuffer(long, ByteBuffer)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_evalBuffer<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    script: JObject<'a>,
) -> JObject<'a> {
    let byte_buffer = JByteBuffer::from(script);
    let address = _env.get_direct_buffer_address(&byte_buffer).unwrap();

    let result = if address.is_null() {
        warn!("script buffer address is null");
        JObject::null()
    } else {
        let byte_buffer_size = _env.get_direct_buffer_capacity(&byte_buffer).unwrap();
        let script_buf: &mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(address, byte_buffer_size) };

        eval(_env, _obj, context_ptr, script_buf)
    };
    result
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.invoke(long, String, Object... args)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_invoke<'a>(
    mut _env: JNIEnv<'a>,
    obj: JObject<'a>,
    context_ptr: jlong,
    name: JString<'a>,
    args: JObjectArray<'a>,
) -> JObject<'a> {
    let context = ptr_to_context(context_ptr);
    let function_name: String = _env
        .get_string(&name)
        .expect("Couldn't get java string!")
        .into();

    let r = context.with(move |ctx| {
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
                            _env.throw_new(
                                "java/lang/Exception",
                                format!("{} is not an object", part),
                            )
                            .unwrap();
                            return JObject::null();
                        }
                    }
                    Err(e) => {
                        handle_exception(e, &ctx, &obj, &mut _env);
                        return JObject::null();
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
                    let result = foreign_function::invoke_js_function_with_java_parameters(
                        _env, &obj, func, args,
                    );
                    result
                } else {
                    _env.throw_new(
                        "java/lang/Exception",
                        format!("{} is not a function", function_name),
                    )
                    .unwrap();
                    JObject::null()
                }
            }
            Err(e) => {
                handle_exception(e, &ctx, &obj, &mut _env);
                JObject::null()
            }
        }
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
    r
}
