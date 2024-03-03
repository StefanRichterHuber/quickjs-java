use jni::{
    objects::{JObject, JValue},
    signature::ReturnType,
    sys::{jint, jlong},
    JNIEnv,
};
use log::{debug, Level, LevelFilter};
use rquickjs::Runtime;

// ---------------------- com.github.stefanrichterhuber.quickjs.QuickJSRuntime
/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSRuntime.createRuntime()
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSRuntime_createRuntime<
    'a,
>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
) -> jlong {
    debug!("Created new QuickJS runtime");
    Box::into_raw(Box::new(Runtime::new().unwrap())) as jlong
}

/// Converts a pointer to a runtime back to a Box<Runtime>.
pub(crate) fn ptr_to_runtime(runtime_ptr: jlong) -> Box<Runtime> {
    let runtime = unsafe { Box::from_raw(runtime_ptr as *mut Runtime) };
    runtime
}

pub(crate) fn runtime_to_ptr(runtime: Box<Runtime>) -> jlong {
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
    debug!("Closed QuickJS runtime");
    let runtime = ptr_to_runtime(runtime_ptr);
    drop(runtime);
}

struct JavaLogContext {
    method_id: jni::objects::JStaticMethodID,
    vm: jni::JavaVM,
    level: Level,
}

/// Implementation of `log::Log` for JavaLogContext. All log messages are passed to the corresponding java method.
impl log::Log for JavaLogContext {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            // Only do JVM call if message would be logged at all
            let method_id = self.method_id;

            let mut env: JNIEnv<'_> = self.vm.get_env().unwrap();
            let level_int = record.level() as i32;
            let message = format!("{} {}", record.metadata().target(), record.args());
            let message_string = env.new_string(message).unwrap();

            let _result = unsafe {
                env.call_static_method_unchecked(
                    "com/github/stefanrichterhuber/quickjs/QuickJSRuntime",
                    method_id,
                    ReturnType::Primitive(jni::signature::Primitive::Void),
                    &[
                        JValue::Int(level_int).as_jni(),
                        JValue::Object(&message_string).as_jni(),
                    ],
                )
            }
            .unwrap();
        }
    }

    fn flush(&self) {
        // Nothing to do here
    }
}

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSRuntime.initLogging()
/// Configures the `log` crate with `std` features to call back to java with each log message
/// see https://docs.rs/log/latest/log/
#[no_mangle]
pub extern "system" fn Java_com_github_stefanrichterhuber_quickjs_QuickJSRuntime_initLogging<'a>(
    mut _env: JNIEnv<'a>,
    _obj: JObject<'a>,
    level: jint,
) {
    let log_id = _env
        .get_static_method_id(
            "com/github/stefanrichterhuber/quickjs/QuickJSRuntime",
            "runtimeLog",
            "(ILjava/lang/String;)V",
        )
        .unwrap();

    let vm = _env.get_java_vm().unwrap();

    let lvl = match level {
        1 => Level::Error,
        2 => Level::Warn,
        3 => Level::Info,
        4 => Level::Debug,
        5 => Level::Trace,
        _ => Level::Error,
    };
    let filter = match level {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Error,
    };

    let log_context = JavaLogContext {
        method_id: log_id,
        vm,
        level: lvl,
    };

    println!("Logger initalized with level {}", level);
    log::set_boxed_logger(Box::new(log_context))
        .map(|()| log::set_max_level(filter))
        .unwrap();
}
