use jni::objects::JValue;
use jni::{objects::JObject, signature::ReturnType, sys::jlong, JNIEnv};
use log::trace;
use log::{debug, error};
use rquickjs::atom::PredefinedAtom;
use rquickjs::object::ObjectKeysIter;
use rquickjs::{Atom, FromAtom, FromJs, Persistent, Value};

use crate::js_array;
use crate::js_object;

/// This proxy assist in converting JS values to Java values
pub struct JSJavaProxy<'js> {
    pub value: Value<'js>,
}

impl<'js> FromJs<'js> for JSJavaProxy<'js> {
    fn from_js(_ctx: &rquickjs::Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        Ok(JSJavaProxy::new(value))
    }
}

impl<'js> FromAtom<'js> for JSJavaProxy<'js> {
    fn from_atom(atom: Atom<'js>) -> rquickjs::Result<Self> {
        Ok(JSJavaProxy::new(atom.to_value()?))
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
            trace!("Map JS array to Java com.github.stefanrichterhuber.quickjs.QuickJSArray");

            let quickjs_array_class = env
                .find_class("com/github/stefanrichterhuber/quickjs/QuickJSArray")
                .expect("Failed to load the target class");

            let array = self.value.into_array().unwrap();
            let ctx = array.ctx().clone();
            let persistent = Persistent::save(&ctx, array);
            let array_ptr = js_array::persistent_to_ptr(Box::new(persistent));

            let quickjs_array = env
                .new_object(
                    quickjs_array_class,
                    "(JLcom/github/stefanrichterhuber/quickjs/QuickJSContext;)V",
                    &[
                        jni::objects::JValueGen::Long(array_ptr as jlong),
                        jni::objects::JValueGen::Object(context),
                    ],
                )
                .unwrap();

            Some(quickjs_array)
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
            debug!("Map JS object to Java com.github.stefanrichterhuber.quickjs.QuickJSObject");

            let quickjs_object_class = env
                .find_class("com/github/stefanrichterhuber/quickjs/QuickJSObject")
                .expect("Failed to load the target class");

            let object = self.value.into_object().unwrap();
            let ctx = object.ctx().clone();
            let persistent = Persistent::save(&ctx, object);
            let object_ptr = js_object::persistent_to_ptr(Box::new(persistent));

            let quickjs_object = env
                .new_object(
                    quickjs_object_class,
                    "(JLcom/github/stefanrichterhuber/quickjs/QuickJSContext;)V",
                    &[
                        jni::objects::JValueGen::Long(object_ptr as jlong),
                        jni::objects::JValueGen::Object(context),
                    ],
                )
                .unwrap();

            return Some(quickjs_object);
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

/// Creates a java.util.Set from a JS object's keys
///
/// # Arguments
///
/// * `context` - The context object
/// * `env` - The JNI environment
/// * `object_keys` - The iterator over the object's keys
///
/// # Returns
///
/// A Java Set containing the object's keys
pub(crate) fn create_java_set_from_object_keys_iter<'js, 'vm>(
    context: &JObject<'vm>,
    env: &mut JNIEnv<'vm>,
    object_keys: ObjectKeysIter<'js, JSJavaProxy<'js>>,
) -> JObject<'vm> {
    let set_class = env
        .find_class("java/util/HashSet")
        .expect("Failed to load the target class");

    let set = env.new_object(&set_class, "()V", &[]).unwrap();
    let add_id = env
        .get_method_id(&set_class, "add", "(Ljava/lang/Object;)Z")
        .unwrap();

    for key in object_keys.into_iter() {
        let key: JSJavaProxy = key.unwrap();
        let key_value = key.into_jobject(context, env).unwrap();
        // Call HashSet.add(Object)

        unsafe {
            env.call_method_unchecked(
                &set,
                add_id,
                ReturnType::Primitive(jni::signature::Primitive::Boolean),
                &[JValue::Object(&key_value).as_jni()],
            )
            .unwrap()
        };
    }

    set
}
