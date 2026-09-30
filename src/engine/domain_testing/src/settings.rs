use domain::settings::{SettingsRecord, SettingsStore};

/// Keeps nothing: the settings start from their defaults.
pub struct StubSettingsStore;

impl SettingsStore for StubSettingsStore {
    fn load(&mut self) -> Option<SettingsRecord> {
        None
    }

    fn save(&mut self, _: &SettingsRecord) {}
}
