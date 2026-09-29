use std::sync::Arc;
use parking_lot::RwLock;

use super::structs::Settings;
use super::{latest_settings, update_settings};

// centralized settings manager.
// wraps Arc<RwLock<Settings>> and handles locking + auto-save
// can be used anywhere, ex. from GUI, tray, IPC, CLI, etc.
#[derive(Clone)]
pub struct SettingsManager {
    inner: Arc<RwLock<Settings>>,
}

impl SettingsManager {
    pub fn new(settings: Settings) -> Self {
        Self {
            inner: Arc::new(RwLock::new(settings)),
        }
    }

    // wrap an existing Arc<RwLock<Settings>>
    pub fn from_arc(arc: Arc<RwLock<Settings>>) -> Self {
        Self { inner: arc }
    }

    // read a setting by key
    pub fn read(&self, key: &str) -> Option<String> {
        let mut settings = self.inner.write();
        if let Some(latest) = latest_settings() {
            *settings = latest;
        }
        settings.get(key)
    }

    // write a setting by key, auto-saves to disk
    pub fn write(&self, key: &str, val: &str) -> Result<(), String> {
        let mut settings = self.inner.write();
        *settings = update_settings(&settings, |latest| latest.set(key, val))?;
        Ok(())
    }

    // write multiple settings at once, single save
    pub fn write_many(&self, pairs: &[(&str, &str)]) -> Result<(), String> {
        let mut settings = self.inner.write();
        *settings = update_settings(&settings, |latest| {
            for (key, val) in pairs {
                latest.set(key, val)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    // direct read access to the full Settings struct (for init code that
    // needs to read multiple fields at once without key-based access)
    pub fn lock(&self) -> parking_lot::RwLockReadGuard<'_, Settings> {
        self.inner.read()
    }

    // direct write access (for bulk operations not covered by set())
    pub fn lock_mut(&self) -> parking_lot::RwLockWriteGuard<'_, Settings> {
        self.inner.write()
    }

    // get the underlying Arc
    pub fn arc(&self) -> &Arc<RwLock<Settings>> {
        &self.inner
    }

    // dump all settings as key-value pairs (for debugging)
    pub fn dump(&self) -> Vec<(String, String)> {
        let settings = self.inner.read();
        Settings::keys().iter()
            .filter_map(|&key| {
                settings.get(key).map(|val| (key.to_string(), val))
            })
            .collect()
    }
}
