use std::sync::Mutex;

use jni::{
    objects::{JObject, JValue},
    signature::ReturnType,
    sys::{jint, jlong},
    JNIEnv,
};
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
    log(LogLevel::DEBUG, "Created new QuickJS runtime");
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
    log(LogLevel::DEBUG, "Closed QuickJS runtime");
    let runtime = ptr_to_runtime(runtime_ptr);
    drop(runtime);
}

struct LogContext {
    method_id: jni::objects::JStaticMethodID,
    vm: jni::JavaVM,
    level: i32,
}

pub enum LogLevel {
    TRACE = 1,
    DEBUG = 2,
    INFO = 3,
    WARN = 4,
    ERROR = 5,
    FATAL = 6,
}

impl LogLevel {
    fn to_string(&self) -> String {
        match self {
            LogLevel::TRACE => "TRACE".to_string(),
            LogLevel::DEBUG => "DEBUG".to_string(),
            LogLevel::INFO => "INFO".to_string(),
            LogLevel::WARN => "WARN".to_string(),
            LogLevel::ERROR => "ERROR".to_string(),
            LogLevel::FATAL => "FATAL".to_string(),
        }
    }
}

static LOG_CONTEXT: Mutex<Option<LogContext>> = Mutex::new(None);

/// Implementation com.github.stefanrichterhuber.quickjs.QuickJSRuntime.initLogging()
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

    LOG_CONTEXT.lock().unwrap().replace(LogContext {
        method_id: log_id,
        vm,
        level,
    });
}

/// Utility function called for logging. If configured, passes the logging to the java function. If not use println! as fallback.
pub fn log(level: LogLevel, message: &str) {
    let mut log_context = LOG_CONTEXT.lock().unwrap();

    if log_context.is_none() {
        // No log context set -> fall back to println!

        println!("[QuickJS native logging] {} {}", level.to_string(), message);
    } else {
        // Log context set -> use it

        let log_context = log_context.as_mut().unwrap();

        let target_level = log_context.level;
        let level_int = level as i32;
        // Only do JVM call if message would be logged at all
        if target_level != 0 && target_level <= level_int {
            let method_id = log_context.method_id;

            let mut env: JNIEnv<'_> = log_context.vm.get_env().unwrap();
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
}
