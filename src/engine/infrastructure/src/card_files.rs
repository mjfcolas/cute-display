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

    fn read_bytes(&self, name: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Unavailable> {
        let path = self.path(name);
        self.storage.read_range(&path, offset, max_bytes).map_err(|fault| Unavailable(format!("{path}: {fault}")))
    }

    fn names_in(&self, directory: &str) -> Result<Vec<String>, Unavailable> {
        let path = self.path(directory);
        let entries = self.storage.entries(&path).map_err(|fault| Unavailable(format!("{path}: {fault}")))?;
        let mut names: Vec<String> = entries.unwrap_or_default().into_iter().filter(|e| !e.is_dir).map(|e| e.name).collect();
        names.sort();
        Ok(names)
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

    fn read_bytes(&self, name: &str, _: u64, _: usize) -> Result<Option<Vec<u8>>, Unavailable> {
        Err(Unavailable(format!("{name}: no SD card ({})", self.0)))
    }

    fn names_in(&self, directory: &str) -> Result<Vec<String>, Unavailable> {
        Err(Unavailable(format!("{directory}: no SD card ({})", self.0)))
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
    fn an_app_lists_a_directory_of_its_own_and_reads_its_files_a_part_at_a_time() {
        let storage = MemoryStorage::default();
        storage.put("cute-display/apps/alarm/ringtones/Zen.mp3", b"0123456789");
        storage.put("cute-display/apps/alarm/ringtones/Default.mp3", b"");
        storage.put("cute-display/apps/alarm/ringtones/old/Harp.mp3", b"");
        storage.put("sounds/alarm/Lost Ark.mp3", b"");
        let files = CardFiles::new(storage, AppId::new("alarm"));
        assert_eq!(files.names_in("ringtones").unwrap(), ["Default.mp3", "Zen.mp3"]);
        assert_eq!(files.names_in("songs").unwrap(), Vec::<String>::new());
        assert_eq!(files.read_bytes("ringtones/Zen.mp3", 8, 4).unwrap().as_deref(), Some(&b"89"[..]));
        assert_eq!(files.read_bytes("ringtones/Harp.mp3", 0, 4).unwrap(), None);
    }

    #[test]
    fn without_the_card_an_app_is_told_why_rather_than_its_files_missing() {
        let files = UnreachableCard(Fault::new("not mounted"));
        let unread = files.read("alarm.conf").unwrap_err();
        assert!(unread.0.contains("not mounted"), "{unread}");
        assert!(files.write("alarm.conf", "enabled = yes").is_err());
        assert!(files.read_bytes("ringtones/Zen.mp3", 0, 4).is_err());
        assert!(files.names_in("ringtones").is_err());
    }
}
