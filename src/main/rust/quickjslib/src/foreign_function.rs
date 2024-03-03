use jni::{
    objects::{JObject, JObjectArray},
    sys::jlong,
    JNIEnv,
};
use log::debug;
use rquickjs::{function::Args, Function};

use crate::{java_js_proxy, js_java_proxy::JSJavaProxy};

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

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSFunction.closeFunction(long ptr)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSFunction_callFunction<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    runtime_ptr: jlong,
    _values: JObjectArray,
) -> JObject<'a> {
    let func = ptr_to_function(runtime_ptr);

    let ctx = func.ctx();

    let args_len = _env.get_array_length(&_values).unwrap();

    let s: Result<JSJavaProxy, _> = if args_len > 0 {
        let mut args = Vec::with_capacity(args_len as usize);
        for i in 0..args_len {
            let arg = _env.get_object_array_element(&_values, i).unwrap();
            let arg_js = java_js_proxy::ProxiedJavaValue::from_object(&mut _env, arg);
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
        Ok(s) => s.into_jobject(&mut _env).unwrap(),
        Err(e) => {
            match e {
                rquickjs::Error::Exception => {
                    let catch = ctx.catch();
                    let execp = catch.as_exception().unwrap();
                    let msg = format!("{:?}", execp);

                    _env.throw_new("java/lang/Exception", msg).unwrap();
                }
                _ => {
                    _env.throw_new("java/lang/Exception", e.to_string())
                        .unwrap();
                }
            }
            JObject::null()
        }
    };

    // Prevents dropping the function
    _ = function_to_ptr(func);
    result
}
