use crate::{
    context::{self, context_to_ptr, ptr_to_context},
    java_js_proxy,
    js_java_proxy::JSJavaProxy,
    with_locale,
};
use jni::{
    objects::{JObject, JObjectArray},
    sys::{jboolean, jint, jlong},
    JNIEnv,
};
use log::trace;
use rquickjs::{function::Args, Array, Context, Function};

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_closeArray<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
) {
    trace!("Closed QuickJSArray with id {}", array_ptr);
    let runtime = ptr_to_jsarray(array_ptr);
    drop(runtime);
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_getArraySize<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    runtime_ptr: jlong,
) -> jint {
    let array = ptr_to_jsarray(runtime_ptr);
    trace!("Called QuickJSArray.getArraySize() with id {}", runtime_ptr);

    let result = array.len() as jint;

    // Prevents dropping the array
    _ = jsarray_to_ptr(array);
    result
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_createNativeArray<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    ctx: JObject<'a>,
) -> jlong {
    let context = context::get_context_from_quickjs_context(&mut _env, ctx);

    let result = context.with(|ctx| {
        let array = create_jsarray(&ctx);
        trace!("Called QuickJSArray.createNativeArray()");
        let result = jsarray_to_ptr(array);
        result
    });

    // Prevents dropping the context
    _ = context_to_ptr(context);

    result as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_setValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    ctx: JObject<'a>,
    index: jint,
    value: JObject<'a>,
) -> jboolean {
    let array = ptr_to_jsarray(array_ptr);

    let value = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &ctx, value);
    array.set(index as usize, value).unwrap();

    // Prevents dropping the array
    _ = jsarray_to_ptr(array);

    true as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_getValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    ctx: JObject<'a>,
    index: jint,
) -> JObject<'a> {
    let array = ptr_to_jsarray(array_ptr);
    let context = context::get_context_from_quickjs_context(&mut env, ctx);

    let s: Result<JSJavaProxy, _> = array.get(index as usize);
    let value = match s {
        Ok(s) => s.into_jobject(&_obj, &mut env).unwrap(),
        Err(e) => {
            context.with(|ctx| {
                context::handle_exception(e, &ctx, &_obj, &mut env);
            });
            JObject::null()
        }
    };

    // Prevents dropping the array
    _ = jsarray_to_ptr(array);

    // Prevents dropping the context
    _ = context_to_ptr(context);

    value
}

/// Converts a raw pointer to a array back to a Box<Array>.
pub(crate) fn ptr_to_jsarray<'js>(array_ptr: jlong) -> Box<Array<'js>> {
    unsafe { Box::from_raw(array_ptr as *mut Array<'js>) }
}

/// Converts a Box<Array> to a raw pointer.
pub(crate) fn jsarray_to_ptr<'js>(array: Box<Array<'js>>) -> jlong {
    Box::into_raw(array) as jlong
}

/// Creates a new JS array.
pub(crate) fn create_jsarray<'js>(ctx: &rquickjs::Ctx<'js>) -> Box<Array<'js>> {
    let obj = rquickjs::Array::new(ctx.clone()).unwrap();
    Box::new(obj)
}
