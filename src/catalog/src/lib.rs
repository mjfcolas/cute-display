use hal::display::Frame;
use ui::Installable;

pub const APPS: &[Installable<Frame>] = &[
    #[cfg(feature = "alarm")]
    alarm::app(),
    #[cfg(feature = "weather")]
    weather::app(),
    #[cfg(feature = "radar")]
    radar::app(),
];

#[cfg(all(test, feature = "alarm", feature = "weather", feature = "radar"))]
mod tests {
    use super::*;

    const INSTALLER_APPS: &str = "tools/installer/src/cute_display_installer/config/apps.py";

    #[test]
    fn the_installer_offers_every_app_by_its_name_and_title() {
        let listed: Vec<String> = APPS.iter().map(|app| format!("'{}': '{}'", app.id.name(), app.title)).collect();
        let expected = format!("TITLES = {{{}}}", listed.join(", "));
        let installer = include_str!("../../../tools/installer/src/cute_display_installer/config/apps.py");
        assert!(installer.lines().any(|line| line == expected), "{INSTALLER_APPS} should say: {expected}");
    }
}
