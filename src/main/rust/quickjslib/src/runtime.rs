use jni::{
    bind_java_type,
    objects::JClass,
    sys::{jint, jlong},
    Env, JavaVM,
};
use log::{debug, warn, Level, LevelFilter};
use rquickjs::Runtime;

use crate::handles::RuntimeHandle;

bind_java_type! {
    rust_type = pub JQuickJSRuntime,
    java_type = "io.github.stefanrichterhuber.quickjs.QuickJSRuntime",

    type_map = {
        unsafe RuntimeHandle => long,
    },

    methods {
        fn js_interrupt() -> jboolean,
        static fn runtime_log(level: jint, message: JString),
    },

    native_methods {
        extern fn create_runtime() -> jlong,
        extern static fn close_runtime(handle: RuntimeHandle),
        extern static fn init_logging(level: jint),
        extern static fn set_memory_limit(handle: RuntimeHandle, limit: jlong),
        extern static fn set_max_stack_size(handle: RuntimeHandle, size: jlong),
    },
}

impl JQuickJSRuntimeNativeInterface for JQuickJSRuntimeAPI {
    type Error = jni::errors::Error;

    fn create_runtime<'local>(
        env: &mut Env<'local>,
        this: JQuickJSRuntime<'local>,
    ) -> Result<jlong, Self::Error> {
        debug!("Created new QuickJS runtime");

        let runtime = Runtime::new().expect("Failed to create QuickJS runtime");

        // Configure callback to runtime to allow java to interrupt running JS script
        let target = env.new_global_ref(&this)?;
        let vm = env.get_java_vm()?;

        let handler = move || {
            vm.attach_current_thread(|env| target.js_interrupt(env))
                .unwrap_or(false)
        };

        runtime.set_interrupt_handler(Some(Box::new(handler)));

        Ok(RuntimeHandle::new(runtime).into())
    }

    fn close_runtime<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: RuntimeHandle,
    ) -> Result<(), Self::Error> {
        debug!("Closed QuickJS runtime");
        drop(unsafe { handle.into_box() });
        Ok(())
    }

    fn init_logging<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        level: jint,
    ) -> Result<(), Self::Error> {
        init_logging(env, level)
    }

    fn set_memory_limit<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: RuntimeHandle,
        limit: jlong,
    ) -> Result<(), Self::Error> {
        debug!("Setting QuickJS runtime memory limit to {} bytes", limit);
        unsafe { handle.as_ref() }.set_memory_limit(limit as usize);
        Ok(())
    }

    fn set_max_stack_size<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        handle: RuntimeHandle,
        size: jlong,
    ) -> Result<(), Self::Error> {
        debug!("Setting QuickJS runtime stack size limit to {} bytes", size);
        unsafe { handle.as_ref() }.set_max_stack_size(size as usize);
        Ok(())
    }
}

/// Base for custom `log::Log` implementation, which allows delegating log output to the Java runtime.
struct JavaLogContext {
    vm: JavaVM,
    level: Level,
}

/// Implementation of `log::Log` for JavaLogContext. All log messages are passed to the corresponding java method.
impl log::Log for JavaLogContext {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if metadata.target().starts_with("jni::") {
            // Tracing from the jni crate itself won't be forwarded to java because it creates an endless loop
            false
        } else {
            metadata.level() <= self.level
        }
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            // Only do JVM call if message would be logged at all.
            // Attaching (instead of reusing the current frame) stashes any pending Java exception while logging
            let _result: Result<(), jni::errors::Error> = self.vm.attach_current_thread(|env| {
                let level = record.level() as jint;
                let message = env.new_string(format!(
                    "{} {}",
                    record.metadata().target(),
                    record.args()
                ))?;
                JQuickJSRuntime::runtime_log(env, level, message)
            });
        }
    }

    fn flush(&self) {
        // Nothing to do here
    }
}

/// Configures the `log` crate with `std` features to call back to java with each log message
/// see https://docs.rs/log/latest/log/
fn init_logging(env: &mut Env<'_>, level: jint) -> Result<(), jni::errors::Error> {
    if level > 0 {
        let vm = env.get_java_vm()?;

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

        let log_context = JavaLogContext { vm, level: lvl };

        match log::set_boxed_logger(Box::new(log_context)).map(|()| log::set_max_level(filter)) {
            Ok(_) => debug!("Native logging redirect to java log4j initialized"),
            Err(_) => warn!("Native logging redirect already configured. Second configuration fails"),
        };
    }
    Ok(())
}
