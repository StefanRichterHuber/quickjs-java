use jni::{
    objects::{JObject, JObjectArray},
    sys::jlong,
    JNIEnv,
};
use log::debug;
use rquickjs::{function::Args, Function};

use crate::{context, java_js_proxy, js_java_proxy::JSJavaProxy};

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSFunction.closeFunction(long ptr)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSFunction_closeFunction<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    function_ptr: jlong,
) {
    debug!("Closed QuickJSFunction with id {}", function_ptr);
    let runtime = ptr_to_function(function_ptr);
    drop(runtime);
}

/// Converts a pointer to a function back to a Box<Function>.
pub(crate) fn ptr_to_function(fun_ptr: jlong) -> Box<Function<'static>> {
    let runtime = unsafe { Box::from_raw(fun_ptr as *mut Function) };
    runtime
}

pub(crate) fn function_to_ptr(fun: Box<Function>) -> jlong {
    Box::into_raw(fun) as jlong
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSFunction.callFunction(long ptr)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSFunction_callFunction<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    runtime_ptr: jlong,
    _values: JObjectArray<'a>,
) -> JObject<'a> {
    // Fetch context object from QuickJS Function
    let context = _env
        .get_field(
            &_obj,
            "ctx",
            "Lcom/github/stefanrichterhuber/quickjs/QuickJSContext;",
        )
        .unwrap()
        .l()
        .unwrap();

    let func = ptr_to_function(runtime_ptr);
    debug!("Called QuickJSFunction with id {}", runtime_ptr);
    let result = invoke_js_function_with_java_parameters(_env, &context, &*func, _values);

    // Prevents dropping the function
    _ = function_to_ptr(func);
    result
}

/// Invokes a JS function with Java parameters. All java parameters are converted to their JS representations, then the function is called.
pub(crate) fn invoke_js_function_with_java_parameters<'a>(
    mut env: JNIEnv<'a>,
    context: &JObject<'a>,
    func: &Function<'_>,
    parameters: JObjectArray<'a>,
) -> JObject<'a> {
    let ctx = func.ctx();

    let args_len = env.get_array_length(&parameters).unwrap();

    let s: Result<JSJavaProxy, _> = if args_len > 0 {
        let mut args = Vec::with_capacity(args_len as usize);
        for i in 0..args_len {
            let arg = env.get_object_array_element(&parameters, i).unwrap();
            let arg_js = java_js_proxy::ProxiedJavaValue::from_object(&mut env, &context, arg);
            args.push(arg_js);
        }

        let mut args_js = Args::new(ctx.clone(), args.len());

        for arg_js in args.into_iter() {
            args_js.push_arg(arg_js).unwrap();
        }
        func.call_arg(args_js)
    } else {
        func.call(())
    };

    let result = match s {
        Ok(s) => s.into_jobject(&context, &mut env).unwrap(),
        Err(e) => {
            context::handle_exception(e, ctx, &mut env);
            JObject::null()
        }
    };
    result
}
