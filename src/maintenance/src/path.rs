//! Paths on the card. Everything may be read; only the device's own directory may be
//! written, so everything of ours is in one place.

pub const DEVICE_DIR: &str = "cute-display";

/// A path relative to the root of the card, `""` being the root itself.
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

    /// Inside the device's own directory, and not the directory itself.
    pub fn is_writable(&self) -> bool {
        self.0.strip_prefix(DEVICE_DIR).and_then(|rest| rest.strip_prefix('/')).is_some_and(|rest| !rest.is_empty())
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
    fn only_files_in_the_devices_directory_are_writable() {
        let writable = |p| SdPath::parse(p).unwrap().is_writable();
        assert!(writable("cute-display/wifi.conf"));
        assert!(writable("cute-display/apps/weather.conf"));
        assert!(!writable("cute-display"));
        assert!(!writable("cute-display.conf"));
        assert!(!writable("cute-displayx/a"));
        assert!(!writable("sounds/alarm.wav"));
    }
}
