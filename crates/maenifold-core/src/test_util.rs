use crate::config::Config;

#[cfg(test)]
pub(crate) struct ConfigResetGuard;

#[cfg(test)]
impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}
