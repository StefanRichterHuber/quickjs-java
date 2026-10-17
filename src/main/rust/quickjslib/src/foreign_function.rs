use jni::{
    bind_java_type,
    objects::{JClass, JObject, JObjectArray},
    Env,
};
use log::trace;
use rquickjs::{function::Args, Function};

use crate::{
    context::{self, JQuickJSContext},
    handles::FunctionHandle,
    java_js_proxy,
    js_java_proxy::JSJavaProxy,
    with_locale,
};

bind_java_type! {
    rust_type = pub JQuickJSFunction,
    java_type = "io.github.stefanrichterhuber.quickjs.QuickJSFunction",

    type_map = {
        JQuickJSContext => "io.github.stefanrichterhuber.quickjs.QuickJSContext",
        unsafe FunctionHandle => long,
    },

    fields {
        ptr: jlong,
        ctx: JQuickJSContext,
    },

    constructors {
        fn new(ptr: jlong, name: JString, context: JQuickJSContext),
    },

    native_methods {
        extern static fn close_function(handle: FunctionHandle),
        extern fn call_function(handle: FunctionHandle, args: JObject[]) -> JObject,
    },
}

bind_java_type! {
    rust_type = pub JVariadicFunction,
    java_type = "io.github.stefanrichterhuber.quickjs.VariadicFunction",

    methods {
        fn apply(args: JObject[]) -> JObject,
    },
}

impl JQuickJSFunctionNativeInterface for JQuickJSFunctionAPI {
    type Error = jni::errors::Error;

    fn close_function<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: FunctionHandle,
    ) -> Result<(), Self::Error> {
        trace!("Closed QuickJSFunction {:?}", handle);
        drop(unsafe { handle.into_box() });
        Ok(())
    }

    fn call_function<'local>(
        env: &mut Env<'local>,
        this: JQuickJSFunction<'local>,
        handle: FunctionHandle,
        args: JObjectArray<'local>,
    ) -> Result<JObject<'local>, Self::Error> {
        // Fetch context object from QuickJS Function
        let context = this.ctx(env)?;

        let func = unsafe { handle.as_ref() };
        trace!("Called QuickJSFunction {:?}", handle);
        invoke_js_function_with_java_parameters(env, &context, func, &args)
    }
}

/// Invokes a JS function with Java parameters. All java parameters are converted to their JS representations, then the function is called.
pub(crate) fn invoke_js_function_with_java_parameters<'a>(
    env: &mut Env<'a>,
    context_object: &JQuickJSContext<'_>,
    func: &Function<'_>,
    parameters: &JObjectArray<'_>,
) -> jni::errors::Result<JObject<'a>> {
    let ctx = func.ctx();
    let locale = with_locale::TemporaryLocale::new_default();

    let args_len = parameters.len(env)?;

    let s: Result<JSJavaProxy, _> = if args_len > 0 {
        let mut args = Vec::with_capacity(args_len);
        for i in 0..args_len {
            let arg = parameters.get_element(env, i)?;
            let arg_js = java_js_proxy::ProxiedJavaValue::from_object(env, context_object, arg);

            let arg_js = match arg_js {
                Ok(arg_js) => arg_js,
                Err(e) => {
                    context::throw_script_exception(env, e.to_string());
                    return Ok(JObject::null());
                }
            };
            args.push(arg_js);
        }

        let mut args_js = Args::new(ctx.clone(), args.len());

        for arg_js in args.into_iter() {
            args_js.push_arg(arg_js).unwrap();
        }
        locale.with(|| func.call_arg(args_js))
    } else {
        locale.with(|| func.call(()))
    };

    let result = match s {
        Ok(s) => s.into_jobject(context_object, env).unwrap(),
        Err(e) => {
            context::handle_exception(e, ctx, env);
            JObject::null()
        }
    };
    Ok(result)
}
