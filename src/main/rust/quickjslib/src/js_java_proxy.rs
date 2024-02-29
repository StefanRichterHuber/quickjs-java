use jni::{objects::JObject, JNIEnv};
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
    // Converts the stored JS value to an Java object
    pub fn into_jobject(self, env: &mut JNIEnv<'vm>) -> Option<JObject<'vm>> {
        if self.value.is_function() {
            println!("JS value is a function -> return not possible");
            return Some(JObject::null());
        } else if self.value.is_object() {
            println!("JS value is an object");

            let ctx = self.value.ctx();
            let value: String = ctx
                .json_stringify(&self.value)
                .unwrap()
                .unwrap()
                .get()
                .unwrap();
            let object = env.new_string(value).unwrap().into();

            return Some(object);
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
