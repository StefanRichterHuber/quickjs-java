//! Typed native handles. Each handle wraps the address of a boxed native object, which is passed to Java as `long`.
//! The address is stored as `jlong` (not as raw pointer), so the handle has the size of a `jlong` on 32-bit targets (e.g. armv7) as well.
//! They are mapped to Java `long` in the `type_map` of the class bindings, so native methods receive typed handles instead of raw `jlong` values.

use std::marker::PhantomData;

use jni::sys::jlong;
use rquickjs::{Array, Context, Function, Object, Persistent, Runtime};

macro_rules! native_handle {
    ($(#[$meta:meta])* $name:ident => $target:ty) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Copy, Clone, Debug)]
        pub struct $name(jlong, PhantomData<*mut $target>);

        impl $name {
            /// Moves the value to the heap and returns a handle to it. The value lives until `into_box` is called and the box dropped.
            pub fn new(value: $target) -> Self {
                Self::from_raw(Box::into_raw(Box::new(value)) as usize as jlong)
            }

            /// Creates a handle from a raw pointer value previously created by `new` (e.g. read from a java field).
            pub fn from_raw(ptr: jlong) -> Self {
                $name(ptr, PhantomData)
            }

            /// Returns a reference to the underlying value.
            ///
            /// # Safety
            /// The caller must ensure that the handle is valid and the value it points to has not been dropped.
            pub unsafe fn as_ref<'a>(&self) -> &'a $target {
                unsafe { &*self.as_ptr() }
            }

            /// Converts the handle back into a box, transferring the ownership of the value.
            ///
            /// # Safety
            /// The caller must ensure that the handle is valid and has not already been freed. The handle must not be used afterwards.
            pub unsafe fn into_box(self) -> Box<$target> {
                unsafe { Box::from_raw(self.as_ptr()) }
            }

            fn as_ptr(&self) -> *mut $target {
                self.0 as usize as *mut $target
            }
        }

        impl From<$name> for jlong {
            fn from(handle: $name) -> Self {
                handle.0
            }
        }
    };
}

native_handle! {
    /// Handle to a QuickJS runtime (`QuickJSRuntime.ptr`)
    RuntimeHandle => Runtime
}

native_handle! {
    /// Handle to a QuickJS context (`QuickJSContext.ptr`)
    ContextHandle => Context
}

native_handle! {
    /// Handle to a JS function (`QuickJSFunction.ptr`)
    FunctionHandle => Function<'static>
}

native_handle! {
    /// Handle to a JS array (`QuickJSArray.ptr`)
    ArrayHandle => Persistent<Array<'static>>
}

native_handle! {
    /// Handle to a JS object (`QuickJSObject.ptr`)
    ObjectHandle => Persistent<Object<'static>>
}
