use jni::{objects::JObject, sys::jlong, JNIEnv};
use log::{error, trace};
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
        if self.value.is_null() {
            trace!("Map JS null to Java null");
            return Some(JObject::null());
        } else if self.value.is_undefined() {
            trace!("Map JS undefined to Java null");
            return Some(JObject::null());
        } else if self.value.is_array() {
            trace!("Map JS array to Java java.util.ArrayList",);
            let array = self.value.as_array().unwrap();
            let len = array.len() as i32;

            let list_class = env
                .find_class("java/util/ArrayList")
                .expect("Failed to load the target class");
            let list = env
                .new_object(list_class, "(I)V", &[jni::objects::JValueGen::Int(len)])
                .unwrap();

            for value in array.iter::<JSJavaProxy>() {
                let value = value.unwrap();
                let value = value.into_jobject(env);

                // TODO cache method id for better performance
                if let Some(v) = value {
                    env.call_method(
                        &list,
                        "add",
                        "(Ljava/lang/Object;)Z",
                        &[jni::objects::JValue::Object(&v)],
                    )
                    .unwrap();
                }
            }

            return Some(list);
        } else if self.value.is_function() {
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
            trace!("Map JS function to Java com.github.stefanrichterhuber.quickjs.QuickJSFunction with id {}", ptr
            );
            return Some(result);
        } else if self.value.is_object() {
            trace!("Map JS object to Java java.util.HashMap",);
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
            trace!("Map JS float to Java java.lang.Double",);

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
            trace!("Map JS int to Java java.lang.Integer",);

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
            trace!("Map JS string to Java java.lang.String",);
            let value: String = self.value.as_string().unwrap().get().unwrap();
            let object = env.new_string(value).unwrap().into();

            return Some(object);
        } else if self.value.is_bool() {
            trace!("Map JS bool to Java java.lang.Boolean",);
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
            error!("Unknown JS value -> could not be mapped!",);
            return None;
        }
    }
}
