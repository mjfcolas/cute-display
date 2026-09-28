use domain::apps::AppId;
use domain::fetch::Unavailable;
use domain::files::Files;
use hal::storage::FileStorage;
use hal::Fault;

pub struct CardFiles<S> {
    storage: S,
    directory: String,
}

impl<S: FileStorage> CardFiles<S> {
    pub fn new(storage: S, app: AppId) -> Self {
        Self { storage, directory: format!("cute-display/apps/{}", app.name()) }
    }

    fn path(&self, name: &str) -> String {
        format!("{}/{name}", self.directory)
    }
}

impl<S: FileStorage + Send> Files for CardFiles<S> {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        let path = self.path(name);
        let bytes = self.storage.read(&path).map_err(|fault| Unavailable(format!("{path}: {fault}")))?;
        Ok(bytes.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable> {
        let path = self.path(name);
        self.storage.write(&path, text.as_bytes()).map_err(|fault| Unavailable(format!("{path}: {fault}")))
    }
}

pub struct UnreachableCard(pub Fault);

impl Files for UnreachableCard {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        Err(Unavailable(format!("{name}: no SD card ({})", self.0)))
    }

    fn write(&self, name: &str, _: &str) -> Result<(), Unavailable> {
        Err(Unavailable(format!("{name}: no SD card ({})", self.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_storage::MemoryStorage;

    #[test]
    fn an_app_reads_and_writes_its_files_in_its_own_directory() {
        let storage = MemoryStorage::with("cute-display/apps/alarm/alarm.conf", "enabled = yes\n");
        let files = CardFiles::new(storage.clone(), AppId::new("alarm"));
        assert_eq!(files.read("alarm.conf").unwrap().as_deref(), Some("enabled = yes\n"));
        assert_eq!(files.read("notes.txt").unwrap(), None);
        files.write("notes.txt", "hello").unwrap();
        assert!(storage.contains("cute-display/apps/alarm/notes.txt"));
    }

    #[test]
    fn without_the_card_an_app_is_told_why_rather_than_its_files_missing() {
        let files = UnreachableCard(Fault::new("not mounted"));
        let unread = files.read("alarm.conf").unwrap_err();
        assert!(unread.0.contains("not mounted"), "{unread}");
        assert!(files.write("alarm.conf", "enabled = yes").is_err());
    }
}
