use jni::{
    objects::{JObject, JString},
    sys::{jint, jlong},
    JNIEnv,
};
use rquickjs::{Context, FromJs, Function, Runtime, Value};
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

/// This proxy assist in converting JS values to Java values
struct JSJavaProxy<'js> {
    value: Value<'js>,
}

impl<'js, 'vm, 'r> JSJavaProxy<'js> {
    pub fn into_jobject(self, env: &mut JNIEnv<'vm>) -> Option<JObject<'vm>> {
        if self.value.is_function() {
            println!("Return value of the script is a function -> return not possible");
            return Some(JObject::null());
        } else if self.value.is_object() {
            println!("Return value of the script is an object");

            let ctx = self.value.ctx();
            let value: String = ctx
                .json_stringify(&self.value)
                .unwrap()
                .unwrap()
                .get()
                .unwrap();
            let object = env.new_string(value).unwrap().into();

            return Some(object);
        } else if self.value.is_float() {
            println!("Return value of the script is a float");

            let value = self.value.as_float().unwrap();
            let class = env
                .find_class("java/lang/Double")
                .expect("Failed to load the target class");
            let result = env
                .call_static_method(
                    class,
                    "valueOf",
                    "(D)Ljava/lang/Double;",
                    &[jni::objects::JValueGen::Double(value)],
                )
                .expect("Failed to create Integer object from value");
            let object = result.l().unwrap();
            return Some(object);
        } else if self.value.is_int() {
            println!("Return value of the script is an int");

            let value = self.value.as_int().unwrap();
            let class = env
                .find_class("java/lang/Integer")
                .expect("Failed to load the target class");
            let result = env
                .call_static_method(
                    class,
                    "valueOf",
                    "(I)Ljava/lang/Integer;",
                    &[jni::objects::JValueGen::Int(value)],
                )
                .expect("Failed to create Integer object from value");
            let object = result.l().unwrap();
            return Some(object);
        } else if self.value.is_string() {
            println!("Return value of the script is a string");
            let value: String = self.value.as_string().unwrap().get().unwrap();
            let object = env.new_string(value).unwrap().into();

            return Some(object);
        } else if self.value.is_null() {
            println!("Return value of the script is a null value");
            return Some(JObject::null());
        } else if self.value.is_undefined() {
            println!("Return value of the script is a undefined value");
            return Some(JObject::null());
        } else if self.value.is_bool() {
            println!("Return value of the script is a boolean");
            let value = self.value.as_bool().unwrap();
            let class = env
                .find_class("java/lang/Boolean")
                .expect("Failed to load the target class");
            let field = if value { "TRUE" } else { "FALSE" };
            let result = env
                .get_static_field(class, field, "Ljava/lang/Boolean;")
                .unwrap();

            let object = result.l().unwrap();
            return Some(object);
        } else {
            println!(
                "Return value of the script is unknown: {}",
                self.value.as_raw().tag
            );
            return None;
        }
    }
}

impl<'js> FromJs<'js> for JSJavaProxy<'js> {
    fn from_js(_ctx: &rquickjs::Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        Ok(JSJavaProxy { value })
    }
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSContext.eval(long, String)
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSContext_eval<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    context_ptr: jlong,
    script: JString<'a>,
) -> JObject<'a> {
    let context = ptr_to_context(context_ptr);
    let script_string: String = _env
        .get_string(&script)
        .expect("Couldn't get java string!")
        .into();

    let r = context.with(move |ctx| {
        let s: Result<JSJavaProxy, _> = ctx.eval(script_string);
        match s {
            Ok(s) => s.into_jobject(&mut _env).unwrap(),
            Err(e) => {
                _env.throw(e.to_string()).unwrap();
                JObject::null()
            }
        }
    });
    // Prevents dropping the context
    _ = context_to_ptr(context);
    r
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
