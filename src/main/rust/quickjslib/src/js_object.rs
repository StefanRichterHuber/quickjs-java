use crate::{
    context::{self, JQuickJSContext},
    handles::ObjectHandle,
    java_js_proxy,
    js_java_proxy::{self, JSJavaProxy},
};
use jni::{
    bind_java_type,
    objects::{JClass, JObject, JSet},
    sys::{jboolean, jint, jlong},
    Env,
};
use log::trace;
use rquickjs::{object::ObjectKeysIter, Ctx, Object, Persistent};

bind_java_type! {
    rust_type = pub JQuickJSObject,
    java_type = "io.github.stefanrichterhuber.quickjs.QuickJSObject",

    type_map = {
        JQuickJSContext => "io.github.stefanrichterhuber.quickjs.QuickJSContext",
        unsafe ObjectHandle => long,
    },

    fields {
        ptr: jlong,
    },

    constructors {
        fn new(ptr: jlong, ctx: JQuickJSContext),
    },

    native_methods {
        extern static fn close_object(handle: ObjectHandle),
        extern static fn get_object_size(handle: ObjectHandle, ctx: JQuickJSContext) -> jint,
        extern static fn create_native_object(ctx: JQuickJSContext) -> jlong,
        extern static fn set_value(handle: ObjectHandle, ctx: JQuickJSContext, key: JObject, value: JObject) -> jboolean,
        extern static fn get_value(handle: ObjectHandle, ctx: JQuickJSContext, key: JObject) -> JObject,
        extern static fn contains_key(handle: ObjectHandle, ctx: JQuickJSContext, key: JObject) -> jboolean,
        extern static fn remove_value(handle: ObjectHandle, ctx: JQuickJSContext, key: JObject) -> jboolean,
        extern static fn key_set(handle: ObjectHandle, ctx: JQuickJSContext) -> JSet,
    },
}

/// Helper function to get a mutable reference to the object
///
/// # Arguments
///
/// * `env` - The JNI environment
/// * `handle` - The handle of the persistent object
/// * `context_object` - The context object
/// * `f` - The function to call with the object
///
/// # Returns
///
/// The result of the function
fn with_object<'java, F, R>(
    env: &mut Env<'java>,
    handle: ObjectHandle,
    context_object: &JQuickJSContext<'_>,
    f: F,
) -> jni::errors::Result<R>
where
    for<'js> F: FnOnce(&mut Env<'java>, Ctx<'js>, Object<'js>) -> R,
    R: 'java,
{
    let object = unsafe { handle.as_ref() };
    let context = context::get_context_from_quickjs_context(env, context_object)?;

    Ok(context::with_context(context, |ctx| {
        let obj = object.clone().restore(&ctx).unwrap();
        f(env, ctx, obj)
    }))
}

/// Converts the given Java key into a JS value. Throws a `QuickJSScriptException` and returns `None` if not possible.
fn convert_key<'local>(
    env: &mut Env<'local>,
    ctx: &JQuickJSContext<'_>,
    key: JObject<'local>,
) -> Option<java_js_proxy::ProxiedJavaValue> {
    match java_js_proxy::ProxiedJavaValue::from_object(env, ctx, key) {
        Ok(key) => Some(key),
        Err(e) => {
            context::throw_script_exception(env, e.to_string());
            None
        }
    }
}

impl JQuickJSObjectNativeInterface for JQuickJSObjectAPI {
    type Error = jni::errors::Error;

    fn close_object<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ObjectHandle,
    ) -> Result<(), Self::Error> {
        trace!("Closed QuickJSObject {:?}", handle);
        drop(unsafe { handle.into_box() });
        Ok(())
    }

    fn get_object_size<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ObjectHandle,
        ctx: JQuickJSContext<'local>,
    ) -> Result<jint, Self::Error> {
        with_object(env, handle, &ctx, |_env, _ctx, object| object.len() as jint)
    }

    fn create_native_object<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        ctx: JQuickJSContext<'local>,
    ) -> Result<jlong, Self::Error> {
        let context = context::get_context_from_quickjs_context(env, &ctx)?;

        let result = context::with_context(context, |ctx| {
            let js_object = rquickjs::Object::new(ctx.clone()).unwrap();
            Persistent::save(&ctx, js_object)
        });

        trace!("Called QuickJSObject.createNativeObject()");
        Ok(ObjectHandle::new(result).into())
    }

    fn set_value<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ObjectHandle,
        ctx: JQuickJSContext<'local>,
        key: JObject<'local>,
        value: JObject<'local>,
    ) -> Result<jboolean, Self::Error> {
        let Some(value) = convert_key(env, &ctx, value) else {
            return Ok(false);
        };
        let Some(key) = convert_key(env, &ctx, key) else {
            return Ok(false);
        };

        with_object(env, handle, &ctx, |env, ctx, object| {
            if let Err(e) = object.set(key, value) {
                context::handle_exception(e, &ctx, env);
            }
        })?;

        Ok(true)
    }

    fn get_value<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ObjectHandle,
        ctx: JQuickJSContext<'local>,
        key: JObject<'local>,
    ) -> Result<JObject<'local>, Self::Error> {
        let Some(key) = convert_key(env, &ctx, key) else {
            return Ok(JObject::null());
        };

        with_object(env, handle, &ctx, |env, js_ctx, object| {
            let s: Result<JSJavaProxy, _> = object.get(key);
            match s {
                Ok(s) => s.into_jobject(&ctx, env).unwrap(),
                Err(e) => {
                    context::handle_exception(e, &js_ctx, env);
                    JObject::null()
                }
            }
        })
    }

    fn contains_key<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ObjectHandle,
        ctx: JQuickJSContext<'local>,
        key: JObject<'local>,
    ) -> Result<jboolean, Self::Error> {
        let Some(key) = convert_key(env, &ctx, key) else {
            return Ok(false);
        };

        with_object(env, handle, &ctx, |env, ctx, object| {
            match object.contains_key(key) {
                Ok(s) => s,
                Err(e) => {
                    context::handle_exception(e, &ctx, env);
                    false
                }
            }
        })
    }

    fn remove_value<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ObjectHandle,
        ctx: JQuickJSContext<'local>,
        key: JObject<'local>,
    ) -> Result<jboolean, Self::Error> {
        let Some(key) = convert_key(env, &ctx, key) else {
            return Ok(false);
        };

        with_object(env, handle, &ctx, |env, ctx, object| {
            match object.remove(key) {
                Ok(_) => true,
                Err(e) => {
                    context::handle_exception(e, &ctx, env);
                    false
                }
            }
        })
    }

    fn key_set<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ObjectHandle,
        ctx: JQuickJSContext<'local>,
    ) -> Result<JSet<'local>, Self::Error> {
        let set = with_object(env, handle, &ctx, |env, _js_ctx, object| {
            let object_keys: ObjectKeysIter<'_, JSJavaProxy<'_>> = object.keys();
            js_java_proxy::create_java_set_from_object_keys_iter(&ctx, env, object_keys)
        })??;
        Ok(set.into())
    }
}
