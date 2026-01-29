use crate::{
    context::{self, context_to_ptr},
    java_js_proxy,
    js_java_proxy::{self, JSJavaProxy},
};
use jni::{
    objects::JObject,
    sys::{jboolean, jint, jlong},
    JNIEnv,
};
use log::trace;
use rquickjs::{object::ObjectKeysIter, Ctx, Object, Persistent};

/// Helper function to get a mutable reference to the object
fn with_object<'java, F, R>(
    mut _env: JNIEnv<'java>,
    object_ptr: jlong,
    context_object: JObject<'java>,
    f: F,
) -> R
where
    for<'js> F: FnOnce(&mut JNIEnv<'java>, JObject<'java>, Ctx<'js>, Object<'js>) -> R,
    R: 'java,
{
    let object_ptr = ptr_to_persistent(object_ptr);
    let context = context::get_context_from_quickjs_context(&mut _env, &context_object);

    let result = context.with(|ctx| {
        let obj = object_ptr.clone().restore(&ctx).unwrap();
        f(&mut _env, context_object, ctx, obj)
    });

    // Prevents dropping the array
    _ = persistent_to_ptr(object_ptr);

    // Prevents dropping the context
    _ = context_to_ptr(context);
    result
}

/// Converts a pointer to a persistent array
pub(crate) fn ptr_to_persistent<'js>(array_ptr: jlong) -> Box<Persistent<Object<'static>>> {
    unsafe { Box::from_raw(array_ptr as *mut Persistent<Object<'static>>) }
}

/// Converts a persistent array to a pointer
pub(crate) fn persistent_to_ptr<'js>(array: Box<Persistent<Object<'static>>>) -> jlong {
    Box::into_raw(array) as jlong
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.createNativeObject
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_createNativeObject<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    ctx: JObject<'a>,
) -> jlong {
    let context = context::get_context_from_quickjs_context(&mut _env, &ctx);

    let result = context.with(|ctx| {
        let js_object = rquickjs::Object::new(ctx.clone()).unwrap();
        let persistent = Persistent::save(&ctx, js_object);
        persistent
    });

    trace!("Called QuickJSObject.createNativeObject()");
    let result = Box::new(result);

    // Prevents dropping the context
    _ = context_to_ptr(context);

    persistent_to_ptr(result)
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.closeObject
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_closeObject<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    object_ptr: jlong,
) {
    trace!("Closed QuickJSObject with id {}", object_ptr);
    let runtime = ptr_to_persistent(object_ptr);
    drop(runtime);
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.getObjectSize
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_getObjectSize<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    object_ptr: jlong,
    ctx: JObject<'a>,
) -> jint {
    let result = with_object(_env, object_ptr, ctx, |mut _env, _, _, object| {
        object.len() as jint
    });
    result
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.setValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_setValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    object_ptr: jlong,
    ctx: JObject<'a>,
    key: JObject<'a>,
    value: JObject<'a>,
) -> jboolean {
    let value = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &ctx, value);
    let key = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &ctx, key);
    with_object(
        env,
        object_ptr,
        ctx,
        |mut _env, context_object, _ctx, object| {
            let s: Result<(), _> = object.set(key, value);
            if s.is_err() {
                context::handle_exception(s.err().unwrap(), &_ctx, &context_object, &mut _env);
            }
        },
    );

    true as jboolean
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.getValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_getValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    object_ptr: jlong,
    context_object: JObject<'a>,
    key: JObject<'a>,
) -> JObject<'a> {
    let key = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &context_object, key);
    let value = with_object(
        env,
        object_ptr,
        context_object,
        |mut env, context_object, _ctx, object| {
            let s: Result<JSJavaProxy, _> = object.get(key).unwrap();
            let value = match s {
                Ok(s) => s.into_jobject(&context_object, &mut env).unwrap(),
                Err(e) => {
                    context::handle_exception(e, &_ctx, &context_object, &mut env);
                    JObject::null()
                }
            };
            value
        },
    );

    value
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.containsKey
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_containsKey<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    object_ptr: jlong,
    context_object: JObject<'a>,
    key: JObject<'a>,
) -> jboolean {
    let key = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &context_object, key);
    let value = with_object(
        env,
        object_ptr,
        context_object,
        |mut env, context_object, _ctx, object| {
            let s: Result<bool, _> = object.contains_key(key);
            match s {
                Ok(s) => s,
                Err(e) => {
                    context::handle_exception(e, &_ctx, &context_object, &mut env);
                    false
                }
            }
        },
    );

    value as jboolean
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.removeValue
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_removeValue<'a>(
    mut env: JNIEnv<'a>,
    _obj: JObject<'a>,
    object_ptr: jlong,
    context_object: JObject<'a>,
    key: JObject<'a>,
) -> jboolean {
    let key = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &context_object, key);
    let value = with_object(
        env,
        object_ptr,
        context_object,
        |mut env, context_object, _ctx, object| {
            let s: Result<(), _> = object.remove(key);
            match s {
                Ok(_) => true,
                Err(e) => {
                    context::handle_exception(e, &_ctx, &context_object, &mut env);
                    false
                }
            }
        },
    );

    value as jboolean
}

/// Implementation of com.github.stefanrichterhuber.quickjs.QuickJSObject.keySet
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSObject_keySet<'vm>(
    env: JNIEnv<'vm>,
    _obj: JObject<'vm>,
    object_ptr: jlong,
    context_object: JObject<'vm>,
) -> JObject<'vm> {
    let value = with_object(
        env,
        object_ptr,
        context_object,
        |env, ctx_object, _ctx, object| {
            let object_keys: ObjectKeysIter<'_, JSJavaProxy<'_>> = object.keys();
            let java_set =
                js_java_proxy::create_java_set_from_object_keys_iter(&ctx_object, env, object_keys);

            java_set
        },
    );

    value
}
