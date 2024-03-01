use std::rc::Rc;

use crate::java_js_proxy::ProxiedJavaValue;
use crate::js_java_proxy::JSJavaProxy;
use crate::runtime::{ptr_to_runtime, runtime_to_ptr};
use jni::{
    errors,
    objects::{JObject, JString, JValueGen},
    sys::{jint, jlong},
    JNIEnv,
};
use rquickjs::{Context, Function, Value};

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
    println!("Created QuickJS context");
    let runtime = ptr_to_runtime(runtime_ptr);
    let context = Context::full(&runtime).unwrap();

    // Prevents dropping the runtime
    _ = runtime_to_ptr(runtime);

    Box::into_raw(Box::new(context)) as jlong
}

/// Converts a pointer to a context back to a Box<Context>.
fn ptr_to_context(context_ptr: jlong) -> Box<Context> {
    let context = unsafe { Box::from_raw(context_ptr as *mut Context) };
    context
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
    println!("Closed QuickJS context");
    let context = ptr_to_context(context_ptr);
    drop(context);
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.setGlobal(long, String, int)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_setGlobal__JLjava_lang_String_2I<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    key: JString<'a>,
    value: jint,
) {
    let context = ptr_to_context(context_ptr);
    let key_string: String = _env
        .get_string(&key)
        .expect("Couldn't get java string!")
        .into();

    let _r = context.with(|ctx| {
        let globals = ctx.globals();
        globals.set(&key_string, value).unwrap();

        println!("Set global [int] var {} = {}", key_string, value);
    });

    // Prevents dropping the context
    _ = context_to_ptr(context);
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.setGlobal(long, String, String)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_setGlobal__JLjava_lang_String_2Ljava_lang_String_2<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    key: JString<'a>,
    value: JString<'a>,
) {
    let context = ptr_to_context(context_ptr);
    let key_string: String = _env
        .get_string(&key)
        .expect("Couldn't get java string!")
        .into();

    let value_string: String = _env
        .get_string(&value)
        .expect("Couldn't get java string!")
        .into();

    let _r = context.with(|ctx| {
        let globals = ctx.globals();
        globals.set(&key_string, &value_string).unwrap();

        println!("Set global [string] var {} = {}", key_string, value_string);
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
            Ok(s) => s.into_jobject(&mut _env).unwrap(),
            Err(e) => {
                _env.throw_new("java/lang/Exception", e.to_string())
                    .unwrap();
                JObject::null()
            }
        }
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
    r
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.setGlobal(long, String, Function<String, String>)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_setGlobal__JLjava_lang_String_2Ljava_util_function_Function_2<
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
    let target = Rc::new(_env.new_global_ref(value).unwrap());

    // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
    let vm = _env.get_java_vm().unwrap();

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

    let _r = context.with(|ctx| {
        let globals = ctx.globals();
        globals
            .set(
                key_string.clone(),
                Function::new(ctx.clone(), f)
                    .unwrap()
                    .with_name(&key_string)
                    .unwrap(),
            )
            .unwrap();

        println!("Set global [function] {}", key_string);
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
}
