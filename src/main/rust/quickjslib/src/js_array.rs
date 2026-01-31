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
use rquickjs::{function::This, Array, Ctx, Function, Persistent};

/// Helper function to get a mutable reference to the array
///
/// # Arguments
///
/// * `_env` - The JNI environment
/// * `array_ptr` - The pointer to the persistent array
/// * `context_object` - The context object
/// * `f` - The function to call with the array
///
/// # Returns
///
/// The result of the function
fn with_array<'java, F, R>(
    mut _env: JNIEnv<'java>,
    array_ptr: jlong,
    context_object: JObject<'java>,
    f: F,
) -> R
where
    for<'js> F: FnOnce(&mut JNIEnv<'java>, JObject<'java>, Ctx<'js>, Array<'js>) -> R,
    R: 'java,
{
    let array_ptr = ptr_to_persistent(array_ptr);
    let context = context::get_context_from_quickjs_context(&mut _env, &context_object);

    let result = context::with_context(&context, |ctx| {
        let array = array_ptr.clone().restore(&ctx).unwrap();
        f(&mut _env, context_object, ctx, array)
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

    let result = context::with_context(&context, |ctx| {
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
    context_object: JObject<'a>,
) -> jint {
    let result = with_array(
        _env,
        array_ptr,
        context_object,
        |mut _env, _context_object, _ctx, array| array.len() as jint,
    );
    result
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.setValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_setValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    context_object: JObject<'a>,
    index: jint,
    value: JObject<'a>,
) -> jboolean {
    let value = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &context_object, value);
    with_array(
        env,
        array_ptr,
        context_object,
        |mut _env, context_object, _ctx, array| {
            let s: Result<(), _> = array.set(index as usize, value);
            if s.is_err() {
                context::handle_exception(s.err().unwrap(), &_ctx, &context_object, &mut _env);
            }
        },
    );

    true as jboolean
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.addValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_addValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    context_object: JObject<'a>,
    index: jint,
    value: JObject<'a>,
) -> jboolean {
    let value = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &context_object, value);
    let result = with_array(
        env,
        array_ptr,
        context_object,
        |mut _env, context_object, _ctx, array| {
            let result = splice_array(array, index, 0, Some(value));
            if result.is_err() {
                context::handle_exception(result.err().unwrap(), &_ctx, &context_object, &mut _env);
                return false as jboolean;
            }
            true as jboolean
        },
    );

    result
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.getValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_getValue<'a>(
    env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    context_object: JObject<'a>,
    index: jint,
) -> JObject<'a> {
    let value = with_array(
        env,
        array_ptr,
        context_object,
        |mut env, ctx_object, _ctx, array| {
            let s: Result<JSJavaProxy, _> = array.get(index as usize);
            let value = match s {
                Ok(s) => s.into_jobject(&ctx_object, &mut env).unwrap(),
                Err(e) => {
                    context::handle_exception(e, &_ctx, &ctx_object, &mut env);
                    JObject::null()
                }
            };
            value
        },
    );

    value
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSArray.removeValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSArray_removeValue<'a>(
    env: JNIEnv<'a>,
    _obj: JObject<'a>,
    array_ptr: jlong,
    context_object: JObject<'a>,
    index: jint,
) -> jboolean {
    let result = with_array(
        env,
        array_ptr,
        context_object,
        |mut env, ctx_object, ctx, array| {
            let result = splice_array(array, index, 1, None);
            if result.is_err() {
                context::handle_exception(result.err().unwrap(), &ctx, &ctx_object, &mut env);
                return false as jboolean;
            }
            true as jboolean
        },
    );

    result
}

/// Helper function to splice an array, by calling the splice method on the array.
///
/// # Arguments
///
/// * `array` - The array to splice
/// * `index` - The index to start splicing from
/// * `delete_count` - The number of elements to delete
/// * `value` - The value to insert
///
/// # Returns
///
/// `Ok(())` if the array was spliced successfully, `Err(e)` if the array was not spliced successfully
fn splice_array<'js>(
    array: Array<'js>,
    index: i32,
    delete_count: i32,
    value: Option<java_js_proxy::ProxiedJavaValue>,
) -> Result<(), rquickjs::Error> {
    let obj = rquickjs::Value::from(array).into_object().unwrap();
    let splice: Function = obj.get("splice")?;
    match value {
        Some(v) => {
            let _s: rquickjs::Value = splice.call((This(obj), index, delete_count, v))?;
        }
        None => {
            let _s: rquickjs::Value = splice.call((This(obj), index, delete_count))?;
        }
    };
    Ok(())
}

/// Converts a pointer to a persistent array
///
/// # Arguments
///
/// * `array_ptr` - The pointer to the persistent array
///
/// # Returns
///
/// A persistent array
pub(crate) fn ptr_to_persistent<'js>(array_ptr: jlong) -> Box<Persistent<Array<'static>>> {
    unsafe { Box::from_raw(array_ptr as *mut Persistent<Array<'static>>) }
}

/// Converts a persistent array to a pointer
///
/// # Arguments
///
/// * `array` - The persistent array to convert
///
/// # Returns
///
/// A pointer to the persistent array
pub(crate) fn persistent_to_ptr<'js>(array: Box<Persistent<Array<'static>>>) -> jlong {
    Box::into_raw(array) as jlong
}

#[cfg(test)]
mod tests {

    use super::*;

    use rquickjs::{Context, Runtime};

    #[test]
    fn test_splice_array() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let array = ctx.eval::<rquickjs::Array, _>("[1, 2, 3]").unwrap();
            let result = splice_array(array, 0, 1, None);
            assert!(result.is_ok());
        });
    }
}
