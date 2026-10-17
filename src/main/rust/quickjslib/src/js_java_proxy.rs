use jni::{objects::JObject, Env};
use log::trace;
use log::{debug, error};
use rquickjs::atom::PredefinedAtom;
use rquickjs::object::ObjectKeysIter;
use rquickjs::{Atom, FromAtom, FromJs, Persistent, Value};

use crate::context::JQuickJSContext;
use crate::foreign_function::JQuickJSFunction;
use crate::handles::{ArrayHandle, FunctionHandle, ObjectHandle};
use crate::java_types::{JBoolean, JDouble, JHashSet, JInteger};
use crate::js_array::JQuickJSArray;
use crate::js_object::JQuickJSObject;

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
    /// - `context` Instance of `io.github.stefanrichterhuber.quickjs.QuickJSContext`, the java object managing the js context.
    pub fn into_jobject(
        self,
        context: &JQuickJSContext<'_>,
        env: &mut Env<'vm>,
    ) -> Option<JObject<'vm>> {
        match self.try_into_jobject(context, env) {
            Ok(object) => Some(object),
            Err(e) => {
                error!("Failed to convert JS value to Java object: {}", e);
                None
            }
        }
    }

    fn try_into_jobject(
        self,
        context: &JQuickJSContext<'_>,
        env: &mut Env<'vm>,
    ) -> jni::errors::Result<JObject<'vm>> {
        if self.value.is_null() {
            trace!("Map JS null to Java null");
            Ok(JObject::null())
        } else if self.value.is_undefined() {
            trace!("Map JS undefined to Java null");
            Ok(JObject::null())
        } else if self.value.is_array() {
            trace!("Map JS array to Java io.github.stefanrichterhuber.quickjs.QuickJSArray");

            let array = self.value.into_array().unwrap();
            let ctx = array.ctx().clone();
            let handle = ArrayHandle::new(Persistent::save(&ctx, array));

            let quickjs_array = JQuickJSArray::new(env, handle.into(), context)?;
            Ok(quickjs_array.into())
        } else if self.value.is_function() {
            let f = self.value.as_function().unwrap().clone();

            let function_name: Result<JSJavaProxy, _> = f.get(PredefinedAtom::Name);
            let function_name = match function_name {
                Ok(s) if s.value.is_string() => {
                    let name: String = s.value.as_string().unwrap().get().unwrap();
                    env.new_string(name)?
                }
                _ => jni::objects::JString::null(),
            };

            // The function is kept alive by the handle until QuickJSFunction is closed -> erase the lifetime
            let f: rquickjs::Function<'static> = unsafe { std::mem::transmute(f) };
            let handle = FunctionHandle::new(f);

            let result = JQuickJSFunction::new(env, handle.into(), &function_name, context)?;
            trace!(
                "Map JS function to Java io.github.stefanrichterhuber.quickjs.QuickJSFunction {:?}",
                handle
            );
            Ok(result.into())
        } else if self.value.is_object() {
            debug!("Map JS object to Java io.github.stefanrichterhuber.quickjs.QuickJSObject");

            let object = self.value.into_object().unwrap();
            let ctx = object.ctx().clone();
            let handle = ObjectHandle::new(Persistent::save(&ctx, object));

            let quickjs_object = JQuickJSObject::new(env, handle.into(), context)?;
            Ok(quickjs_object.into())
        } else if self.value.is_float() {
            trace!("Map JS float to Java java.lang.Double",);
            let value = self.value.as_float().unwrap();
            Ok(JDouble::value_of(env, value)?.into())
        } else if self.value.is_int() {
            trace!("Map JS int to Java java.lang.Integer",);
            let value = self.value.as_int().unwrap();
            Ok(JInteger::value_of(env, value)?.into())
        } else if self.value.is_string() {
            trace!("Map JS string to Java java.lang.String",);
            let value: String = self.value.as_string().unwrap().get().unwrap();
            Ok(env.new_string(value)?.into())
        } else if self.value.is_bool() {
            trace!("Map JS bool to Java java.lang.Boolean",);
            let value = self.value.as_bool().unwrap();
            Ok(JBoolean::value_of(env, value)?.into())
        } else {
            Err(jni::errors::Error::WrongObjectType)
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
    context: &JQuickJSContext<'_>,
    env: &mut Env<'vm>,
    object_keys: ObjectKeysIter<'js, JSJavaProxy<'js>>,
) -> jni::errors::Result<JHashSet<'vm>> {
    let set = JHashSet::new(env)?;

    for key in object_keys.into_iter() {
        let key: JSJavaProxy = key.unwrap();
        let key_value = key.into_jobject(context, env).unwrap();
        set.add(env, &key_value)?;
    }

    Ok(set)
}
