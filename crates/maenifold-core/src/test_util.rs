use crate::config::Config;

#[cfg(test)]
pub(crate) struct ConfigResetGuard;

#[cfg(test)]
static CONFIG_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
thread_local! {
    static CONFIG_TEST_GUARD: std::cell::RefCell<Option<std::sync::MutexGuard<'static, ()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
impl ConfigResetGuard {
    pub(crate) fn acquire_test_lock() {
        let guard = CONFIG_TEST_LOCK.lock().expect("config test lock");
        CONFIG_TEST_GUARD.with(|slot| {
            *slot.borrow_mut() = Some(guard);
        });
    }
}

#[cfg(test)]
impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
        CONFIG_TEST_GUARD.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}
