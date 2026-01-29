use crate::{
    context::{self, context_to_ptr},
    java_js_proxy,
    js_java_proxy::JSJavaProxy,
};
use jni::{
    objects::JObject,
    sys::{jboolean, jint, jlong},
    JNIEnv,
};
use log::trace;
use rquickjs::{Array, Ctx, Persistent};

/// Helper function to get a mutable reference to the array
fn with_array<'java, F, R>(
    mut _env: JNIEnv<'java>,
    array_ptr: jlong,
    ctx: JObject<'java>,
    f: F,
) -> R
where
    for<'js> F: FnOnce(&mut JNIEnv<'java>, Ctx<'js>, Array<'js>) -> R,
    R: 'java,
{
    let array_ptr = ptr_to_persistent(array_ptr);
    let context = context::get_context_from_quickjs_context(&mut _env, &ctx);

    let result = context.with(|ctx| {
        let array = array_ptr.clone().restore(&ctx).unwrap();
        f(&mut _env, ctx, array)
    });

    // Prevents dropping the array
    _ = persistent_to_ptr(array_ptr);

    // Prevents dropping the context
    _ = context_to_ptr(context);
    result
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.createNativeArray
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_createNativeArray<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    ctx: JObject<'a>,
) -> jlong {
    let context = context::get_context_from_quickjs_context(&mut _env, &ctx);

    let result = context.with(|ctx| {
        let js_array = rquickjs::Array::new(ctx.clone()).unwrap();
        let persistent = Persistent::save(&ctx, js_array);
        persistent
    });

    trace!("Called QuickJSArray.createNativeArray()");
    let result = Box::new(result);

    // Prevents dropping the context
    _ = context_to_ptr(context);

    persistent_to_ptr(result)
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.closeArray
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_closeArray<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
) {
    trace!("Closed QuickJSArray with id {}", array_ptr);
    let runtime = ptr_to_persistent(array_ptr);
    drop(runtime);
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.getArraySize
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_getArraySize<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    ctx: JObject<'a>,
) -> jint {
    let result = with_array(_env, array_ptr, ctx, |mut _env, _ctx, array| {
        array.len() as jint
    });
    result
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.setValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_setValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    ctx: JObject<'a>,
    index: jint,
    value: JObject<'a>,
) -> jboolean {
    let value = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &ctx, value);
    with_array(env, array_ptr, ctx, |mut _env, _ctx, array| {
        let s: Result<(), _> = array.set(index as usize, value);
        if s.is_err() {
            context::handle_exception(s.err().unwrap(), &_ctx, &_obj, &mut _env);
        }
    });

    true as jboolean
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.getValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_getValue<'a>(
    env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    ctx: JObject<'a>,
    index: jint,
) -> JObject<'a> {
    let value = with_array(env, array_ptr, ctx, |mut env, _ctx, array| {
        let s: Result<JSJavaProxy, _> = array.get(index as usize).unwrap();
        let value = match s {
            Ok(s) => s.into_jobject(&_obj, &mut env).unwrap(),
            Err(e) => {
                context::handle_exception(e, &_ctx, &_obj, &mut env);
                JObject::null()
            }
        };
        value
    });

    value
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.removeValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_removeValue<'a>(
    env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    ctx: JObject<'a>,
    index: jint,
) -> jboolean {
    let value = with_array(env, array_ptr, ctx, |mut env, _ctx, array| {
        // TODO implement (call js function splice?)
    });

    true as jboolean
}

/// Converts a pointer to a persistent array
pub(crate) fn ptr_to_persistent<'js>(array_ptr: jlong) -> Box<Persistent<Array<'static>>> {
    unsafe { Box::from_raw(array_ptr as *mut Persistent<Array<'static>>) }
}

/// Converts a persistent array to a pointer
pub(crate) fn persistent_to_ptr<'js>(array: Box<Persistent<Array<'static>>>) -> jlong {
    Box::into_raw(array) as jlong
}
