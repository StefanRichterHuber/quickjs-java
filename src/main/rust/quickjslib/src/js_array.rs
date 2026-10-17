use crate::{
    context::{self, JQuickJSContext},
    handles::ArrayHandle,
    java_js_proxy,
    js_java_proxy::JSJavaProxy,
};
use jni::{
    bind_java_type,
    objects::{JClass, JObject},
    sys::{jboolean, jint, jlong},
    Env,
};
use log::trace;
use rquickjs::{function::This, Array, Ctx, Function, Persistent};

bind_java_type! {
    rust_type = pub JQuickJSArray,
    java_type = "io.github.stefanrichterhuber.quickjs.QuickJSArray",

    type_map = {
        JQuickJSContext => "io.github.stefanrichterhuber.quickjs.QuickJSContext",
        unsafe ArrayHandle => long,
    },

    fields {
        ptr: jlong,
    },

    constructors {
        fn new(ptr: jlong, ctx: JQuickJSContext),
    },

    native_methods {
        extern static fn close_array(handle: ArrayHandle),
        extern static fn get_array_size(handle: ArrayHandle, ctx: JQuickJSContext) -> jint,
        extern static fn create_native_array(ctx: JQuickJSContext) -> jlong,
        extern static fn set_value(handle: ArrayHandle, ctx: JQuickJSContext, index: jint, value: JObject) -> jboolean,
        extern static fn add_value(handle: ArrayHandle, ctx: JQuickJSContext, index: jint, value: JObject) -> jboolean,
        extern static fn get_value(handle: ArrayHandle, ctx: JQuickJSContext, index: jint) -> JObject,
        extern static fn remove_value(handle: ArrayHandle, ctx: JQuickJSContext, index: jint) -> jboolean,
    },
}

/// Helper function to get a mutable reference to the array
///
/// # Arguments
///
/// * `env` - The JNI environment
/// * `handle` - The handle of the persistent array
/// * `context_object` - The context object
/// * `f` - The function to call with the array
///
/// # Returns
///
/// The result of the function
fn with_array<'java, F, R>(
    env: &mut Env<'java>,
    handle: ArrayHandle,
    context_object: &JQuickJSContext<'_>,
    f: F,
) -> jni::errors::Result<R>
where
    for<'js> F: FnOnce(&mut Env<'java>, Ctx<'js>, Array<'js>) -> R,
    R: 'java,
{
    let array = unsafe { handle.as_ref() };
    let context = context::get_context_from_quickjs_context(env, context_object)?;

    Ok(context::with_context(context, |ctx| {
        let array = array.clone().restore(&ctx).unwrap();
        f(env, ctx, array)
    }))
}

impl JQuickJSArrayNativeInterface for JQuickJSArrayAPI {
    type Error = jni::errors::Error;

    fn close_array<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ArrayHandle,
    ) -> Result<(), Self::Error> {
        trace!("Closed QuickJSArray {:?}", handle);
        drop(unsafe { handle.into_box() });
        Ok(())
    }

    fn get_array_size<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ArrayHandle,
        ctx: JQuickJSContext<'local>,
    ) -> Result<jint, Self::Error> {
        with_array(env, handle, &ctx, |_env, _ctx, array| array.len() as jint)
    }

    fn create_native_array<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        ctx: JQuickJSContext<'local>,
    ) -> Result<jlong, Self::Error> {
        let context = context::get_context_from_quickjs_context(env, &ctx)?;

        let result = context::with_context(context, |ctx| {
            let js_array = rquickjs::Array::new(ctx.clone()).unwrap();
            Persistent::save(&ctx, js_array)
        });

        trace!("Called QuickJSArray.createNativeArray()");
        Ok(ArrayHandle::new(result).into())
    }

    fn set_value<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ArrayHandle,
        ctx: JQuickJSContext<'local>,
        index: jint,
        value: JObject<'local>,
    ) -> Result<jboolean, Self::Error> {
        let value = match java_js_proxy::ProxiedJavaValue::from_object(env, &ctx, value) {
            Ok(value) => value,
            Err(e) => {
                context::throw_script_exception(env, e.to_string());
                return Ok(false);
            }
        };

        with_array(env, handle, &ctx, |env, ctx, array| {
            if let Err(e) = array.set(index as usize, value) {
                context::handle_exception(e, &ctx, env);
            }
        })?;

        Ok(true)
    }

    fn add_value<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ArrayHandle,
        ctx: JQuickJSContext<'local>,
        index: jint,
        value: JObject<'local>,
    ) -> Result<jboolean, Self::Error> {
        let value = match java_js_proxy::ProxiedJavaValue::from_object(env, &ctx, value) {
            Ok(value) => value,
            Err(e) => {
                context::throw_script_exception(env, e.to_string());
                return Ok(false);
            }
        };

        with_array(env, handle, &ctx, |env, ctx, array| {
            if let Err(e) = splice_array(array, index, 0, Some(value)) {
                context::handle_exception(e, &ctx, env);
                return false;
            }
            true
        })
    }

    fn get_value<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ArrayHandle,
        ctx: JQuickJSContext<'local>,
        index: jint,
    ) -> Result<JObject<'local>, Self::Error> {
        with_array(env, handle, &ctx, |env, js_ctx, array| {
            let s: Result<JSJavaProxy, _> = array.get(index as usize);
            match s {
                Ok(s) => s.into_jobject(&ctx, env).unwrap(),
                Err(e) => {
                    context::handle_exception(e, &js_ctx, env);
                    JObject::null()
                }
            }
        })
    }

    fn remove_value<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: ArrayHandle,
        ctx: JQuickJSContext<'local>,
        index: jint,
    ) -> Result<jboolean, Self::Error> {
        with_array(env, handle, &ctx, |env, ctx, array| {
            if let Err(e) = splice_array(array, index, 1, None) {
                context::handle_exception(e, &ctx, env);
                return false;
            }
            true
        })
    }
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

#[cfg(test)]
mod tests {

    use super::*;

    use rquickjs::{Context, Runtime};

    #[test]
    fn test_splice_array() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let array = ctx
                .eval::<rquickjs::Array, _>(" let a = [1, 2, 3]; a")
                .unwrap();
            let result = splice_array(array, 0, 1, None);
            assert!(result.is_ok());

            let result = ctx.eval::<rquickjs::Array, _>("a").unwrap();
            assert_eq!(result.len(), 2);

            let result = ctx.eval::<rquickjs::Value, _>("a[0]").unwrap();
            assert_eq!(result.as_int().unwrap(), 2);

            let result = ctx.eval::<rquickjs::Value, _>("a[1]").unwrap();
            assert_eq!(result.as_int().unwrap(), 3);
        });
    }
}
