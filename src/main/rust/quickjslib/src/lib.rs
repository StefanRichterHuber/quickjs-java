use std::rc::Rc;

use jni::{
    objects::{JObject, JString},
    sys::{jint, jstring},
    JNIEnv,
};
use rquickjs::{Coerced, Context, Function, Runtime};

static mut RUNTIME: Option<Runtime> = None;
static mut CONTEXT: Option<Context> = None;

fn print(msg: String) {
    println!("Message from JS: {}", msg);
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_App_init<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
) {
    unsafe {
        if RUNTIME.is_none() {
            RUNTIME = Some(Runtime::new().unwrap());
            println!("Initialized runtime");
        }
        if CONTEXT.is_none() {
            CONTEXT = Some(Context::full(RUNTIME.as_ref().unwrap()).unwrap());
            println!("Initialized context");
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_App_setGlobal__Ljava_lang_String_2I<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    key: JString<'a>,
    value: jint,
) {
    let key_string: String = _env
        .get_string(&key)
        .expect("Couldn't get java string!")
        .into();

    unsafe {
        let _r = CONTEXT.as_ref().unwrap().with(|ctx| {
            let globals = ctx.globals();
            globals.set(&key_string, value).unwrap();

            println!("Set global [int] var {} = {}", key_string, value);
        });
    }
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_App_setGlobal__Ljava_lang_String_2Ljava_lang_String_2<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    key: JString<'a>,
    value: JString<'a>,
) {
    let key_string: String = _env
        .get_string(&key)
        .expect("Couldn't get java string!")
        .into();

    let value_string: String = _env
        .get_string(&value)
        .expect("Couldn't get java string!")
        .into();

    unsafe {
        let _r = CONTEXT.as_ref().unwrap().with(|ctx| {
            let globals = ctx.globals();
            globals.set(&key_string, &value_string).unwrap();

            println!("Set global [string] var {} = {}", key_string, value_string);
        });
    }
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_App_setGlobal__Ljava_lang_String_2Ljava_util_function_Function_2<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    key: JString<'a>,
    value: JObject<'a>,
) {
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

    unsafe {
        let _r = CONTEXT.as_ref().unwrap().with(|ctx| {
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
    }
}

#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_App_executeScript<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    x: JString<'a>,
) -> jstring {
    unsafe {
        let r = CONTEXT.as_ref().unwrap().with(|ctx| {
            let input: String = _env
                .get_string(&x)
                .expect("Couldn't get java string!")
                .into();

            let r = ctx.eval::<Coerced<String>, _>(input).unwrap().0;

            _env.new_string(r).unwrap().into_raw()
        });

        r
    }
}
