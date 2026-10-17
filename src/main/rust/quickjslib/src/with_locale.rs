//! Workaround for locale dependent number parsing in QuickJS.
//!
//! QuickJS (quickjs-ng 0.16, also current master) parses numbers in `JSON.parse` with the libc function `strtod`, which
//! uses the decimal separator of the current `LC_NUMERIC` locale (all other number parsing / formatting is locale independent).
//! Starting a JVM sets the locale from the environment, so on systems with ',' as decimal separator (e.g. german systems)
//! `JSON.parse('1.5')` returns `1` (the part after the '.' is just discarded).
//! See <https://github.com/bellard/quickjs/issues/106> and <https://github.com/DelSkayn/rquickjs/issues/281>.
//!
//! The workaround switches the `LC_NUMERIC` locale of the current thread to "C" while JS code is executed (see
//! `context::with_context`). Only the calling thread is affected (`uselocale`), the process wide locale is unchanged.
//! It can be disabled with the 'locale_workaround' feature and is a no-op on windows (no `uselocale`).
//!
//! Remove this module once QuickJS parses JSON numbers with its own locale independent `js_atod`.

/// Executes `f` with the "C" `LC_NUMERIC` locale set for the current thread. The previous locale is restored afterwards, even if `f` panics.
#[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
pub fn with_c_numeric_locale<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    /// Restores the previous locale of the thread when dropped
    struct RestoreLocale(libc::locale_t);

    impl Drop for RestoreLocale {
        fn drop(&mut self) {
            unsafe { libc::uselocale(self.0) };
        }
    }

    match c_numeric_locale() {
        Some(locale) => {
            let _restore = RestoreLocale(unsafe { libc::uselocale(locale) });
            f()
        }
        None => f(),
    }
}

/// No-op if the 'locale_workaround' feature is disabled or on windows.
#[cfg(any(not(feature = "locale_workaround"), target_os = "windows"))]
#[inline(always)]
pub fn with_c_numeric_locale<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    f()
}

/// Returns the "C" `LC_NUMERIC` locale, which is created once and never freed (it is used for the whole lifetime of the library).
#[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
fn c_numeric_locale() -> Option<libc::locale_t> {
    use std::sync::OnceLock;

    /// `locale_t` is a raw pointer. Locale objects can be used by any thread (`uselocale`) and this one is never modified or freed.
    struct SharedLocale(libc::locale_t);
    unsafe impl Send for SharedLocale {}
    unsafe impl Sync for SharedLocale {}

    static LOCALE: OnceLock<Option<SharedLocale>> = OnceLock::new();

    LOCALE
        .get_or_init(|| {
            // The "C" locale is always available (unlike e.g. "en_US.UTF-8") and uses '.' as decimal separator
            let locale =
                unsafe { libc::newlocale(libc::LC_NUMERIC_MASK, c"C".as_ptr(), std::ptr::null_mut()) };
            if locale.is_null() {
                log::warn!("Failed to create 'C' locale, JSON.parse might parse numbers locale dependent");
                None
            } else {
                Some(SharedLocale(locale))
            }
        })
        .as_ref()
        .map(|locale| locale.0)
}

#[cfg(test)]
#[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
pub(crate) mod tests {
    use std::ffi::CStr;

    use super::with_c_numeric_locale;

    /// Sets the given locale for the current thread while executing `f`. Returns `None` (and skips `f`) if the locale is not installed.
    pub(crate) fn with_thread_locale<R>(name: &CStr, f: impl FnOnce() -> R) -> Option<R> {
        let locale = unsafe { libc::newlocale(libc::LC_ALL_MASK, name.as_ptr(), std::ptr::null_mut()) };
        if locale.is_null() {
            eprintln!("Locale {:?} not installed, test skipped", name);
            return None;
        }
        let previous = unsafe { libc::uselocale(locale) };
        let result = f();
        unsafe {
            libc::uselocale(previous);
            libc::freelocale(locale);
        }
        Some(result)
    }

    /// Formats a float with the C library, which respects the `LC_NUMERIC` locale of the thread
    fn format_float(value: f64) -> String {
        let mut buf = [0 as libc::c_char; 64];
        unsafe { libc::snprintf(buf.as_mut_ptr(), buf.len(), c"%.1f".as_ptr(), value) };
        unsafe { CStr::from_ptr(buf.as_ptr()) }.to_str().unwrap().to_string()
    }

    #[test]
    fn test_c_numeric_locale() {
        with_thread_locale(c"de_DE.UTF-8", || {
            assert_eq!("2,2", format_float(2.2));
            assert_eq!("2.2", with_c_numeric_locale(|| format_float(2.2)));
            // Previous locale restored
            assert_eq!("2,2", format_float(2.2));
        });
    }

    #[test]
    fn test_c_numeric_locale_restored_after_panic() {
        with_thread_locale(c"de_DE.UTF-8", || {
            let result = std::panic::catch_unwind(|| with_c_numeric_locale(|| panic!("test")));
            assert!(result.is_err());
            assert_eq!("2,2", format_float(2.2));
        });
    }
}
