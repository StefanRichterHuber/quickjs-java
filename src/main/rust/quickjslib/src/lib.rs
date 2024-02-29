use jni::{
    objects::{JObject, JString},
    sys::{jint, jlong, jstring},
    JNIEnv,
};
use rquickjs::{Coerced, Context, Function, Runtime};
use std::rc::Rc;

// ----------------------------------------------------------------------------------------
// ---------------------- com.github.stefanrichterhuber.quickjs.QuickJSRuntime
/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSRuntime.createRuntime()
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSRuntime_createRuntime<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
) -> jlong {
    println!("Created QuickJS runtime");
    Box::into_raw(Box::new(Runtime::new().unwrap())) as jlong
}

/// Converts a pointer to a runtime back to a Box<Runtime>.
fn ptr_to_runtime(runtime_ptr: jlong) -> Box<Runtime> {
    let runtime = unsafe { Box::from_raw(runtime_ptr as *mut Runtime) };
    runtime
}

fn runtime_to_ptr(runtime: Box<Runtime>) -> jlong {
    Box::into_raw(runtime) as jlong
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSRuntime.closeRuntime(long ptr)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSRuntime_closeRuntime<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    runtime_ptr: jlong,
) {
    println!("Closed QuickJS runtime");
    let runtime = ptr_to_runtime(runtime_ptr);
    drop(runtime);
}

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
) -> jstring {
    let context = ptr_to_context(context_ptr);
    let script_string: String = _env
        .get_string(&script)
        .expect("Couldn't get java string!")
        .into();

    let r = context.with(|ctx| {
        let r = ctx.eval::<Coerced<String>, _>(script_string).unwrap().0;
        r
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
    _env.new_string(r).unwrap().into_raw()
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
    let name = key_string.clone();
    let target = Rc::new(_env.new_global_ref(value).unwrap());

    // https://github.com/jni-rs/jni-rs/issues/488#issuecomment-1699852154
    let vm = _env.get_java_vm().unwrap();

    let f = move |msg: String| {
        println!("Calling function {} with parameter '{}'", name, msg);

        let mut env = vm.get_env().unwrap();
        let param = env.new_string(msg).unwrap();

        let call_result = env.call_method(
            target.as_ref(),
            "apply",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            &[jni::objects::JValueGen::Object(&param)],
        );

        let o = call_result.unwrap().l().unwrap();
        let str: JString = o.into();
        let plain: String = env.get_string(&str).unwrap().into();
        plain
    };

    let _r = context.with(|ctx| {
        let globals = ctx.globals();
        globals
            .set(
                key_string.clone(),
                Function::new(ctx.clone(), f)
                    .unwrap()
                    .with_name("key_string")
                    .unwrap(),
            )
            .unwrap();

        println!("Set global [function] {}", key_string);
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
}
