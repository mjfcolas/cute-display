//! Paths on the card. Everything can be read; only the image's own directory can be
//! written, so that all the files of an image are in one place.

/// A path relative to the root of the card, where `""` is the root itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdPath(String);

impl SdPath {
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        let text = text.trim_end_matches('/');
        if text.starts_with('/') || text.contains('\\') {
            return Err("paths are relative to the root of the card, with '/'");
        }
        if !text.is_empty() && text.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
            return Err("a path may not climb or skip directories");
        }
        Ok(Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Inside `directory`, and not the directory itself. A directory named `""`, or with a
    /// '/' at either end, contains nothing: a wrong name writes nowhere.
    pub fn is_inside(&self, directory: &str) -> bool {
        self.0.strip_prefix(directory).and_then(|rest| rest.strip_prefix('/')).is_some_and(|rest| !rest.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_may_neither_be_absolute_nor_climb() {
        for bad in ["/sdcard/x", "../x", "cute-display/../sounds/a.wav", "a//b", "./a", "a\\b"] {
            assert!(SdPath::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(SdPath::parse("").unwrap().as_str(), "");
        assert_eq!(SdPath::parse("sounds/").unwrap().as_str(), "sounds");
    }

    #[test]
    fn a_path_is_inside_a_directory_below_it_and_not_beside_it() {
        let inside = |p| SdPath::parse(p).unwrap().is_inside("cute-display");
        assert!(inside("cute-display/wifi.conf"));
        assert!(inside("cute-display/apps/weather.conf"));
        assert!(!inside("cute-display"));
        assert!(!inside("cute-display.conf"));
        assert!(!inside("cute-display-e2e/wifi.conf"));
        assert!(!inside("sounds/alarm.wav"));
    }

    #[test]
    fn a_mistaken_directory_has_nothing_inside() {
        for not_one in ["", "cute-display/", "/cute-display"] {
            for path in ["wifi.conf", "cute-display/wifi.conf", "sounds/alarm.wav"] {
                assert!(!SdPath::parse(path).unwrap().is_inside(not_one), "{not_one:?} {path}");
            }
        }
    }
}
