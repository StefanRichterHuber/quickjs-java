use jni::{objects::JObject, sys::jlong, JNIEnv};
use rquickjs::{FromJs, Value};
/// This proxy assist in converting JS values to Java values
pub struct JSJavaProxy<'js> {
    pub value: Value<'js>,
}

impl<'js> FromJs<'js> for JSJavaProxy<'js> {
    fn from_js(_ctx: &rquickjs::Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        Ok(JSJavaProxy { value })
    }
}

impl<'js, 'vm, 'r> JSJavaProxy<'js> {
    pub fn new(value: Value<'js>) -> Self {
        JSJavaProxy { value }
    }

    // Converts the stored JS value to an Java object
    pub fn into_jobject(self, env: &mut JNIEnv<'vm>) -> Option<JObject<'vm>> {
        if self.value.is_null() || self.value.is_undefined() {
            println!("JS value is null or undefined -> return null");
            return Some(JObject::null());
        } else if self.value.is_function() {
            println!("JS value is a function -> convert to com.github.stefanrichterhuber.quickjs.QuickJSFunction");

            let f = self.value.as_function().unwrap();
            let f = f.clone();

            let func = Box::new(f);
            let ptr = Box::into_raw(func) as jlong;

            let js_function_class = env
                .find_class("com/github/stefanrichterhuber/quickjs/QuickJSFunction")
                .expect("Failed to load the target class");

            let result = env
                .new_object(
                    js_function_class,
                    "(J)V",
                    &[jni::objects::JValueGen::Long(ptr)],
                )
                .unwrap();

            return Some(result);
        } else if self.value.is_object() {
            // FIXME convert to map instead
            println!("JS value is an object -> convert to java.util.HashMap");

            let obj = self.value.as_object().unwrap();

            let hash_map_class = env
                .find_class("java/util/HashMap")
                .expect("Failed to load the target class");

            let hash_map = env.new_object(hash_map_class, "()V", &[]).unwrap();

            // TODO cache method id for better performance
            for v in obj.keys() {
                let key: String = v.unwrap();
                let k = env.new_string(&key).unwrap();
                let value: JSJavaProxy = obj.get(key.as_str()).unwrap();
                let value = value.into_jobject(env);

                println!("    Add key: {} to map", key);

                if let Some(v) = value {
                    env.call_method(
                        &hash_map,
                        "put",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                        &[
                            jni::objects::JValueGen::Object(&k),
                            jni::objects::JValueGen::Object(&v),
                        ],
                    )
                    .unwrap();
                }
            }
            return Some(hash_map);
        } else if self.value.is_float() {
            println!("JS value is a float");

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
            println!("JS value is an int");

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
            println!("JS value is a string");
            let value: String = self.value.as_string().unwrap().get().unwrap();
            let object = env.new_string(value).unwrap().into();

            return Some(object);
        } else if self.value.is_null() {
            println!("JS value is a null value");
            return Some(JObject::null());
        } else if self.value.is_undefined() {
            println!("JS value is a undefined value");
            return Some(JObject::null());
        } else if self.value.is_bool() {
            println!("JS value is a boolean");
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
            println!("JS value is unknown: {}", self.value.as_raw().tag);
            return None;
        }
    }
}
