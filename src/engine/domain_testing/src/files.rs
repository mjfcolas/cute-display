use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use domain::fetch::Unavailable;
use domain::files::Files;

use crate::shared::lock;

/// An app's files in memory.
#[derive(Clone, Default)]
pub struct FakeFiles(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

impl FakeFiles {
    pub fn with(name: &str, contents: impl AsRef<[u8]>) -> Self {
        let files = Self::default();
        files.put(name, contents);
        files
    }

    pub fn put(&self, name: &str, contents: impl AsRef<[u8]>) {
        lock(&self.0).insert(name.into(), contents.as_ref().to_vec());
    }

    pub fn file(&self, name: &str) -> Option<Vec<u8>> {
        lock(&self.0).get(name).cloned()
    }
}

impl Files for FakeFiles {
    fn read(&self, name: &str) -> Result<Option<String>, Unavailable> {
        Ok(self.file(name).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn write(&self, name: &str, text: &str) -> Result<(), Unavailable> {
        self.put(name, text);
        Ok(())
    }

    fn read_bytes(&self, name: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Unavailable> {
        let skipped = usize::try_from(offset).unwrap_or(usize::MAX);
        Ok(self.file(name).map(|bytes| bytes.into_iter().skip(skipped).take(max_bytes).collect()))
    }

    fn names_in(&self, directory: &str) -> Result<Vec<String>, Unavailable> {
        let prefix = format!("{directory}/");
        let files = lock(&self.0);
        Ok(files.keys().filter_map(|name| name.strip_prefix(&prefix)).filter(|name| !name.contains('/')).map(String::from).collect())
    }
}

/// Checks an app's files, none at first, against `Files`' contract.
pub fn check_contract(files: &impl Files) {
    assert_eq!(files.read("alarm.conf"), Ok(None), "a file that is not there");
    assert_eq!(files.read_bytes("alarm.conf", 0, 4), Ok(None), "bytes of a file that is not there");
    assert_eq!(files.names_in("ringtones"), Ok(Vec::new()), "a directory that is not there");

    assert_eq!(files.write("alarm.conf", "on = no\n"), Ok(()));
    assert_eq!(files.write("alarm.conf", "on = yes\n"), Ok(()), "writing again replaces");
    assert_eq!(files.read("alarm.conf"), Ok(Some("on = yes\n".into())));
    assert_eq!(files.read_bytes("alarm.conf", 5, 3), Ok(Some(b"yes".to_vec())));
    assert_eq!(files.read_bytes("alarm.conf", 5, 100), Ok(Some(b"yes\n".to_vec())), "fewer towards the end");

    for name in ["ringtones/Zen.mp3", "ringtones/Lost Ark.mp3", "ringtones/old/Bell.mp3"] {
        assert_eq!(files.write(name, "ID3"), Ok(()));
    }
    assert_eq!(files.names_in("ringtones"), Ok(vec!["Lost Ark.mp3".into(), "Zen.mp3".into()]), "the files alone, in order");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_files_keep_the_contract() {
        check_contract(&FakeFiles::default());
    }
}
