#[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
use log::debug;
#[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
use std::ffi::CString;

/// A temporary locale that can be used to temporarily set the locale for a function invocation. This is necessary due to a bug / design decision in QuickJS <https://github.com/bellard/quickjs/issues/106>
/// which makes float parsing dependent on current locale. Starting a JVM sets a locale, so on some systems with ',' as decimal separator (e.g. german systems) parsing float values fails.
/// These values are then recognized as int values (part after the ',' is just discarded) (see <https://github.com/DelSkayn/rquickjs/issues/281>)
/// This workaround can be activated with the 'locale_workaround' feature. It is disabled on windows platforms.
pub struct TemporaryLocale {
    #[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
    locale: libc::locale_t,
}

static DEFAULT_LOCALE: &str = "en_US.UTF-8";

impl TemporaryLocale {
    /// Creates a new temporary locale with the suitable default locale to fulfill the workaround.
    pub fn new_default() -> Self {
        TemporaryLocale::new(DEFAULT_LOCALE)
    }
}

impl Default for TemporaryLocale {
    fn default() -> Self {
        TemporaryLocale::new_default()
    }
}

#[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
impl TemporaryLocale {
    /// Creates a new temporary locale with the given locale (e.g. 'en_US.UTF-8')
    pub fn new(locale: &str) -> Self {
        let locale = CString::new(locale).unwrap();
        let script_locale = unsafe {
            libc::newlocale(libc::LC_NUMERIC_MASK, locale.as_ptr(), std::ptr::null_mut())
        };

        if script_locale.is_null() {
            panic!(
                "Failed to create locale '{}'. Install it with 'sudo locale-gen {}' or similar",
                locale.to_str().unwrap(),
                locale.to_str().unwrap(),
            );
        }

        TemporaryLocale {
            locale: script_locale,
        }
    }

    /// Executes the given function with the temporary locale set.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        debug!("Setting locale temporarily");
        let current_locale = unsafe { libc::uselocale(self.locale) };
        let result = f();
        unsafe { libc::uselocale(current_locale) };
        debug!("Reset to previous locale");
        result
    }
}

impl Drop for TemporaryLocale {
    fn drop(&mut self) {
        #[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
        unsafe {
            libc::freelocale(self.locale)
        };
    }
}

// For a disables feature 'locale_workaround' or building for windows, the workaround is a no op
#[cfg(any(not(feature = "locale_workaround"), target_os = "windows"))]
impl TemporaryLocale {
    /// Creates a new temporary locale with the given locale (e.g. 'en_US.UTF-8')
    pub fn new(_locale: &str) -> Self {
        TemporaryLocale {}
    }

    /// Executes the given function with the temporary locale set.
    #[inline(always)]
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        f()
    }
}

#[cfg(test)]
#[cfg(all(feature = "locale_workaround", not(target_os = "windows")))]
mod tests {
    use std::{ffi::CString, mem};

    use libc::{c_char, sprintf};

    use crate::with_locale::TemporaryLocale;
    #[test]
    fn test_temporary_locale() {
        let format = CString::new("%f").unwrap();

        TemporaryLocale::new("en_US.utf8").with(|| {
            let mut v: Vec<c_char> = Vec::with_capacity(1000);
            let s = v.as_mut_ptr();
            mem::forget(v);

            unsafe {
                sprintf(s, format.as_ptr(), 2.2);
            }

            let result = unsafe { CString::from_raw(s) };

            // The locale is set to en_US, so the decimal separator is '.'
            assert_eq!("2.200000", result.to_str().unwrap());
        });

        TemporaryLocale::new("de_DE.utf8").with(|| {
            let mut v: Vec<c_char> = Vec::with_capacity(1000);
            let s = v.as_mut_ptr();
            mem::forget(v);

            unsafe {
                sprintf(s, format.as_ptr(), 2.2);
            }

            let result = unsafe { CString::from_raw(s) };

            // The locale is set to de_DE, so the decimal separator is ','
            assert_eq!("2,200000", result.to_str().unwrap());
        });
    }
}
