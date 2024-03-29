use crate::foreign_function;
use crate::java_js_proxy::ProxiedJavaValue;
use crate::js_java_proxy::JSJavaProxy;
use crate::runtime::{ptr_to_runtime, runtime_to_ptr};
use jni::objects::JObjectArray;
use jni::{
    objects::{JObject, JString},
    sys::jlong,
    JNIEnv,
};
use log::debug;
use log::trace;
use rquickjs::{Context, Error, Value};

// ----------------------------------------------------------------------------------------

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
                handle_exception(e, &ctx, &mut _env);
                JObject::null()
            }
        }
    });

    // Prevents dropping the context
    _ = context_to_ptr(context);

    result
}

/// Handle JS errors. Extracts the message and throws a Java exception..
pub(crate) fn handle_exception(e: Error, ctx: &rquickjs::Ctx<'_>, _env: &mut JNIEnv<'_>) {
    let msg = match e {
        Error::Exception => {
            let catch = ctx.catch();
            if let Some(execp) = catch.as_exception() {
                format!("{:?}", execp)
            } else if let Some(msg) = catch.as_string() {
                msg.to_string().unwrap()
            } else {
                "Unknown type of JS Error::Exception".to_string()
            }
        }
        _ => e.to_string(),
    };
    _env.throw_new("java/lang/Exception", msg).unwrap();
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
                handle_exception(e, &ctx, &mut _env);
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
    let context = ptr_to_context(context_ptr);
    let script_string: String = _env
        .get_string(&script)
        .expect("Couldn't get java string!")
        .into();

    let r = context.with(move |ctx| {
        let s: Result<JSJavaProxy, _> = ctx.eval(script_string);

        match s {
            Ok(s) => s.into_jobject(&_obj, &mut _env).unwrap(),
            Err(e) => {
                handle_exception(e, &ctx, &mut _env);
                JObject::null()
            }
        }
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
    r
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.invoke(long, String, Object... args)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_invoke<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
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
                        handle_exception(e, &ctx, &mut _env);
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
                        _env, &_obj, func, args,
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
                handle_exception(e, &ctx, &mut _env);
                JObject::null()
            }
        }
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
    r
}
