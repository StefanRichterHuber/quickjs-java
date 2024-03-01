use jni::objects::JThrowable;
use jni::{
    objects::{JObject, JString},
    JNIEnv,
};
use log::info;
use rquickjs::{IntoJs, Value};

pub enum ProxiedJavaValue {
    THROWABLE(String),
    NULL,
    STRING(String),
    DOUBLE(f64),
    INT(i32),
    BOOL(bool),
}

impl ProxiedJavaValue {
    pub fn from_object<'vm>(env: &mut JNIEnv<'vm>, obj: JObject<'vm>) -> Self {
        info!("Calling ProxiedJavaValue::from_object");
        if obj.is_null() {
            println!("Java value is null");
            return ProxiedJavaValue::NULL;
        }

        // TODO implement different types
        let double_class = env
            .find_class("java/lang/Double")
            .expect("Failed to load the target class");
        let float_class = env
            .find_class("java/lang/Float")
            .expect("Failed to load the target class");
        let int_class = env
            .find_class("java/lang/Integer")
            .expect("Failed to load the target class");
        let long_class = env
            .find_class("java/lang/Long")
            .expect("Failed to load the target class");
        let string_class = env
            .find_class("java/lang/String")
            .expect("Failed to load the target class");
        let bool_class = env
            .find_class("java/lang/Boolean")
            .expect("Failed to load the target class");

        if env.is_instance_of(&obj, double_class).unwrap()
            || env.is_instance_of(&obj, float_class).unwrap()
        {
            let raw_value = env.call_method(&obj, "doubleValue", "()D", &[]);
            let value = raw_value.unwrap().d().unwrap();
            println!("Java value is a double or float: {}", value);
            ProxiedJavaValue::DOUBLE(value)
        } else if env.is_instance_of(&obj, string_class).unwrap() {
            let str: JString = obj.into();
            let plain: String = env.get_string(&str).unwrap().into();
            println!("Java value is a string: {}", plain);
            ProxiedJavaValue::STRING(plain)
        } else if env.is_instance_of(&obj, int_class).unwrap()
            || env.is_instance_of(&obj, long_class).unwrap()
        {
            let raw_value = env.call_method(&obj, "intValue", "()I", &[]);
            let value = raw_value.unwrap().i().unwrap();
            println!("Java value is an int or long: {}", value);
            ProxiedJavaValue::INT(value)
        } else if env.is_instance_of(&obj, bool_class).unwrap() {
            let raw_value = env.call_method(&obj, "booleanValue", "()Z", &[]);
            let value = raw_value.unwrap().z().unwrap();
            println!("Java value is a boolean: {}", value);
            ProxiedJavaValue::BOOL(value)
        } else {
            let raw_value = env.call_method(&obj, "toString", "()Ljava/lang/String;", &[]);
            let str: JString = raw_value.unwrap().l().unwrap().into();
            let plain: String = env.get_string(&str).unwrap().into();
            println!("Java value is of unknown type -> toString(): {}", plain);
            ProxiedJavaValue::STRING(plain)
        }
    }

    pub fn from_throwable<'vm>(env: &mut JNIEnv<'vm>, throwable: JThrowable<'vm>) -> Self {
        // @see https://stackoverflow.com/questions/27072459/how-to-get-the-message-from-a-java-exception-caught-in-jni
        // Seems to be necessary, otherwise fetching the message fails
        env.exception_clear().unwrap();
        let message = env.call_method(&throwable, "getMessage", "()Ljava/lang/String;", &[]);
        let str: JString = message.unwrap().l().unwrap().into();
        let error_msg: String = env.get_string(&str).unwrap().into();
        println!("Java value is an exception {}", error_msg);
        ProxiedJavaValue::THROWABLE(error_msg)
    }

    pub fn from_null() -> Self {
        println!("Java value is null");
        ProxiedJavaValue::NULL
    }
}

impl<'js> IntoJs<'js> for ProxiedJavaValue {
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let result = match self {
            ProxiedJavaValue::THROWABLE(_) => Ok(Value::new_null(ctx.clone())),
            ProxiedJavaValue::NULL => Ok(Value::new_null(ctx.clone())),
            ProxiedJavaValue::STRING(str) => Ok(Value::from_string(
                rquickjs::String::from_str(ctx.clone(), str.as_str()).unwrap(),
            )),
            ProxiedJavaValue::DOUBLE(v) => Ok(Value::new_float(ctx.clone(), v)),
            ProxiedJavaValue::INT(v) => Ok(Value::new_int(ctx.clone(), v)),
            ProxiedJavaValue::BOOL(v) => Ok(Value::new_bool(ctx.clone(), v)),
        };
        result
    }
}
