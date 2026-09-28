use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use hal::storage::{CopyOutcome, Entry, FileStorage};
use hal::Fault;

#[derive(Clone)]
pub struct DirectoryCard {
    root: Arc<PathBuf>,
}

impl DirectoryCard {
    pub fn open(root: &Path) -> Result<Self, Fault> {
        fs::create_dir_all(root).map_err(|e| Fault::new(format!("{}: {e}", root.display())))?;
        Ok(Self { root: Arc::new(root.to_path_buf()) })
    }

    /// Only paths inside the card: no `..`, no root of their own.
    fn locate(&self, path: &str) -> Result<PathBuf, Fault> {
        let relative = Path::new(path);
        if relative.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)) {
            Ok(self.root.join(relative))
        } else {
            Err(Fault::new(format!("{path}: not a path on the card")))
        }
    }
}

fn fault(path: &str) -> impl Fn(io::Error) -> Fault + '_ {
    move |e| Fault::new(format!("{path}: {e}"))
}

fn absent_is_none<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

impl FileStorage for DirectoryCard {
    fn entries(&self, dir: &str) -> Result<Option<Vec<Entry>>, Fault> {
        let Some(listing) = absent_is_none(fs::read_dir(self.locate(dir)?)).map_err(fault(dir))? else {
            return Ok(None);
        };
        let mut entries = listing
            .map(|entry| {
                let entry = entry.map_err(fault(dir))?;
                let metadata = entry.metadata().map_err(fault(dir))?;
                Ok(Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    size_bytes: if metadata.is_dir() { 0 } else { metadata.len() },
                    is_dir: metadata.is_dir(),
                })
            })
            .collect::<Result<Vec<_>, Fault>>()?;
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Some(entries))
    }

    fn capacity_bytes(&self) -> Result<u64, Fault> {
        Err(Fault::new("a directory has no capacity of its own"))
    }

    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        absent_is_none(fs::read(self.locate(path)?)).map_err(fault(path))
    }

    fn read_range(&self, path: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Fault> {
        let Some(mut file) = absent_is_none(File::open(self.locate(path)?)).map_err(fault(path))? else {
            return Ok(None);
        };
        let mut contents = Vec::new();
        file.seek(SeekFrom::Start(offset))
            .and_then(|_| file.take(max_bytes as u64).read_to_end(&mut contents))
            .map_err(fault(path))?;
        Ok(Some(contents))
    }

    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault> {
        let file = self.locate(path)?;
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir).map_err(fault(path))?;
        }
        fs::write(file, contents).map_err(fault(path))
    }

    fn copy(&self, from: &str, to: &str) -> Result<CopyOutcome, Fault> {
        let (source, destination) = (self.locate(from)?, self.locate(to)?);
        if !source.is_file() {
            return Ok(CopyOutcome::NoSource);
        }
        if let Some(dir) = destination.parent() {
            fs::create_dir_all(dir).map_err(fault(to))?;
        }
        fs::copy(source, destination).map(|_| CopyOutcome::Copied).map_err(fault(to))
    }

    fn remove(&self, path: &str) -> Result<(), Fault> {
        absent_is_none(fs::remove_file(self.locate(path)?)).map(|_| ()).map_err(fault(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_card(name: &str) -> DirectoryCard {
        let root = std::env::temp_dir().join(format!("cute-display-card-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        DirectoryCard::open(&root).unwrap()
    }

    #[test]
    fn a_file_written_under_new_directories_reads_back() {
        let card = blank_card("round-trip");
        card.write("cute-display/weather.conf", b"place = Paris\n").unwrap();
        assert_eq!(card.read("cute-display/weather.conf").unwrap().as_deref(), Some(&b"place = Paris\n"[..]));
        assert_eq!(card.read_range("cute-display/weather.conf", 8, 3).unwrap().as_deref(), Some(&b"Par"[..]));
        let entries = card.entries("cute-display").unwrap();
        assert_eq!(entries, Some(vec![Entry { name: "weather.conf".into(), size_bytes: 14, is_dir: false }]));
        assert_eq!(card.entries("sounds").unwrap(), None);
    }

    #[test]
    fn a_missing_file_is_none_and_removing_it_is_no_fault() {
        let card = blank_card("missing");
        assert_eq!(card.read("cute-display/none.conf").unwrap(), None);
        assert_eq!(card.read_range("cute-display/none.conf", 0, 10).unwrap(), None);
        assert!(card.remove("cute-display/none.conf").is_ok());
    }

    #[test]
    fn a_file_is_copied_under_new_directories() {
        let card = blank_card("copy");
        card.write("sounds/alarm/Zen.mp3", b"ID3").unwrap();
        assert_eq!(card.copy("sounds/alarm/Zen.mp3", "cute-display/apps/alarm/ringtones/Zen.mp3").unwrap(), CopyOutcome::Copied);
        assert_eq!(card.read("cute-display/apps/alarm/ringtones/Zen.mp3").unwrap().as_deref(), Some(&b"ID3"[..]));
        assert_eq!(card.copy("sounds/alarm/None.mp3", "cute-display/None.mp3").unwrap(), CopyOutcome::NoSource);
    }

    #[test]
    fn paths_that_leave_the_card_are_refused() {
        let card = blank_card("escape");
        assert!(card.read("../outside").is_err());
        assert!(card.write("/etc/outside", b"").is_err());
    }
}
