use hal::storage::{CopyOutcome, Entry, FileStorage};
use hal::Fault;

/// A directory at the root of the card, one name with no separator, such as an image's
/// own. In a const context, as the composition roots name it, a name that is not one
/// directory does not build; called at run time, it would panic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardDirectory(&'static str);

impl CardDirectory {
    pub const fn new(name: &'static str) -> Self {
        let bytes = name.as_bytes();
        assert!(!bytes.is_empty(), "a directory has a name");
        assert!(!matches!(bytes, b"." | b".."), "a directory neither climbs nor stays put");
        let mut rest = bytes;
        while let [byte, others @ ..] = rest {
            assert!(*byte != b'/' && *byte != b'\\', "one directory, at the root of the card");
            rest = others;
        }
        Self(name)
    }

    pub const fn name(self) -> &'static str {
        self.0
    }
}

/// The card seen from an image's own directory, where everything the image keeps is: its
/// paths are relative to it, `""` being the directory itself, and may neither be absolute
/// nor climb out of it.
#[derive(Clone)]
pub struct OwnDirectory<S> {
    card: S,
    directory: CardDirectory,
}

impl<S: FileStorage> OwnDirectory<S> {
    pub fn new(card: S, directory: CardDirectory) -> Self {
        Self { card, directory }
    }

    pub fn path_from_card_root(&self, path: &str) -> String {
        let name = self.directory.name();
        if path.is_empty() {
            name.to_owned()
        } else {
            format!("{name}/{path}")
        }
    }

    fn inside(&self, path: &str) -> Result<String, Fault> {
        if path.starts_with('/') || path.contains('\\') {
            return Err(Fault::new(format!("{path}: paths are relative to {}/, with '/'", self.directory.name())));
        }
        if !path.is_empty() && path.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
            return Err(Fault::new(format!("{path}: a path may not climb or skip directories")));
        }
        Ok(self.path_from_card_root(path))
    }
}

impl<S: FileStorage> FileStorage for OwnDirectory<S> {
    fn entries(&self, dir: &str) -> Result<Option<Vec<Entry>>, Fault> {
        self.card.entries(&self.inside(dir)?)
    }

    fn capacity_bytes(&self) -> Result<u64, Fault> {
        self.card.capacity_bytes()
    }

    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        self.card.read(&self.inside(path)?)
    }

    fn read_range(&self, path: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Fault> {
        self.card.read_range(&self.inside(path)?, offset, max_bytes)
    }

    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault> {
        self.card.write(&self.inside(path)?, contents)
    }

    fn copy(&self, from: &str, to: &str) -> Result<CopyOutcome, Fault> {
        self.card.copy(&self.inside(from)?, &self.inside(to)?)
    }

    fn remove(&self, path: &str) -> Result<(), Fault> {
        self.card.remove(&self.inside(path)?)
    }
}

#[cfg(test)]
mod tests {
    use hal_testing::storage::{self, FakeFileStorage};

    use super::*;

    const APP_IMAGE: CardDirectory = CardDirectory::new("cute-display");
    const TEST_IMAGE: CardDirectory = CardDirectory::new("cute-display-e2e");

    #[test]
    fn an_own_directory_keeps_the_contract() {
        storage::check_contract(&OwnDirectory::new(FakeFileStorage::default(), TEST_IMAGE));
    }

    #[test]
    fn what_an_image_keeps_lands_in_its_directory_and_nowhere_else() {
        let card = FakeFileStorage::with("cute-display/wifi.conf", "ssid = Home\n");
        let tested = OwnDirectory::new(card.clone(), TEST_IMAGE);
        tested.write("wifi.conf", b"ssid = Lab\n").unwrap();
        assert_eq!(card.file("cute-display-e2e/wifi.conf").as_deref(), Some(&b"ssid = Lab\n"[..]));
        assert_eq!(card.file("cute-display/wifi.conf").as_deref(), Some(&b"ssid = Home\n"[..]));
        assert_eq!(OwnDirectory::new(card, APP_IMAGE).read("wifi.conf").unwrap().as_deref(), Some(&b"ssid = Home\n"[..]));
    }

    #[test]
    fn a_path_may_neither_be_absolute_nor_climb_out() {
        let card = FakeFileStorage::with("sounds/alarm/Zen.mp3", "ID3");
        let own = OwnDirectory::new(card, APP_IMAGE);
        for outside in ["/sounds/alarm/Zen.mp3", "../sounds/alarm/Zen.mp3", "apps/../../sounds", "a//b", "./a", "a\\b"] {
            assert!(own.read(outside).is_err(), "{outside}");
            assert!(own.write(outside, b"").is_err(), "{outside}");
        }
        assert!(own.copy("../sounds/alarm/Zen.mp3", "Zen.mp3").is_err());
    }

    #[test]
    fn a_path_is_named_from_the_root_of_the_card() {
        let own = OwnDirectory::new(FakeFileStorage::default(), APP_IMAGE);
        assert_eq!(own.path_from_card_root("wifi.conf"), "cute-display/wifi.conf");
        assert_eq!(own.path_from_card_root(""), "cute-display");
    }

    #[test]
    fn a_directory_is_one_name_at_the_root_of_the_card() {
        assert_eq!(CardDirectory::new("cute-display-e2e").name(), "cute-display-e2e");
        for not_one in ["", ".", "..", "a/b", "/a", "a\\b"] {
            assert!(std::panic::catch_unwind(|| CardDirectory::new(not_one)).is_err(), "{not_one:?}");
        }
    }
}
