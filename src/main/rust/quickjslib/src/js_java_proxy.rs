use jni::objects::JValue;
use jni::{objects::JObject, signature::ReturnType, sys::jlong, JNIEnv};
use log::error;
use log::trace;
use rquickjs::atom::PredefinedAtom;
use rquickjs::{FromJs, Value};

/// This proxy assist in converting JS values to Java values
pub struct JSJavaProxy<'js> {
    pub value: Value<'js>,
}

impl<'js> FromJs<'js> for JSJavaProxy<'js> {
    fn from_js(_ctx: &rquickjs::Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        Ok(JSJavaProxy::new(value))
    }
}

impl<'js, 'vm> JSJavaProxy<'js> {
    /// Creates a new JSJavaProxy from a JS value
    pub fn new(value: Value<'js>) -> Self {
        JSJavaProxy { value }
    }

    /// Converts the stored JS value to an Java object
    /// - `context` Instance of `com.github.stefanrichterhuber.quickjs.QuickJSContext`, the java object managing the js context.
    pub fn into_jobject(
        self,
        context: &JObject<'vm>,

        env: &mut JNIEnv<'vm>,
    ) -> Option<JObject<'vm>> {
        if self.value.is_null() {
            trace!("Map JS null to Java null");
            Some(JObject::null())
        } else if self.value.is_undefined() {
            trace!("Map JS undefined to Java null");
            Some(JObject::null())
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

            let add_id = env
                .get_method_id("java/util/ArrayList", "add", "(Ljava/lang/Object;)Z")
                .unwrap();

            for value in array.iter::<JSJavaProxy>() {
                let value = value.unwrap();
                let value = value.into_jobject(context, env);

                if let Some(v) = value {
                    unsafe {
                        env.call_method_unchecked(
                            &list,
                            add_id,
                            ReturnType::Primitive(jni::signature::Primitive::Boolean),
                            &[JValue::Object(&v).as_jni()],
                        )
                        .unwrap()
                    };
                }
            }

            Some(list)
        } else if self.value.is_function() {
            let f = self.value.as_function().unwrap();
            let f = f.clone();

            let function_name: Result<JSJavaProxy, _> = f.get(PredefinedAtom::Name);
            let function_name = match function_name {
                Ok(s) => s.into_jobject(context, env).unwrap(),
                Err(_) => JObject::null(),
            };

            let func = Box::new(f);
            let ptr = Box::into_raw(func) as jlong;

            let js_function_class = env
                .find_class("com/github/stefanrichterhuber/quickjs/QuickJSFunction")
                .expect("Failed to load the target class");

            let result = env.new_object(
                js_function_class,
                "(JLjava/lang/String;Lcom/github/stefanrichterhuber/quickjs/QuickJSContext;)V",
                &[
                    jni::objects::JValueGen::Long(ptr),
                    jni::objects::JValueGen::Object(&function_name),
                    jni::objects::JValueGen::Object(context),
                ],
            );

            match result {
                Ok(result) => {
                    trace!("Map JS function to Java com.github.stefanrichterhuber.quickjs.QuickJSFunction with id {}", ptr
                    );
                    return Some(result);
                }
                Err(e) => {
                    error!("Failed to create a new object: {}", e);
                    return None;
                }
            }
        } else if self.value.is_object() {
            trace!("Map JS object to Java java.util.HashMap",);
            let obj = self.value.as_object().unwrap();

            let hash_map_class = env
                .find_class("java/util/HashMap")
                .expect("Failed to load the target class");

            let hash_map = env.new_object(hash_map_class, "()V", &[]).unwrap();

            // Determines the method id of the Map.put(K, V) method for better performance
            let put_id = env
                .get_method_id(
                    "java/util/HashMap",
                    "put",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                )
                .unwrap();

            for v in obj.keys() {
                let key: String = v.unwrap();
                let k = env.new_string(&key).unwrap();
                let value: JSJavaProxy = obj.get(key.as_str()).unwrap();
                let value = value.into_jobject(context, env);

                if let Some(v) = value {
                    unsafe {
                        env.call_method_unchecked(
                            &hash_map,
                            put_id,
                            ReturnType::Object,
                            &[JValue::Object(&k).as_jni(), JValue::Object(&v).as_jni()],
                        )
                        .unwrap()
                    };
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
