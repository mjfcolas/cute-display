use crate::bench::{Image, Setup, SETTINGS_CONF};

#[test]
fn a_setting_changed_with_the_controls_is_kept_on_the_card_and_after_a_restart() {
    let image = Image::start(Setup::default());
    image.click_wheel();
    assert_eq!(image.screen_line("front").as_deref(), Some("front system"));
    assert_eq!(image.screen_line("setting").as_deref(), Some("setting Backlight 10 s"));
    image.turn_wheel(3);
    image.press_long_button();
    assert_eq!(image.screen_line("setting").as_deref(), Some("setting Backlight 30 s *"));
    assert!(image.file(SETTINGS_CONF).is_some_and(|conf| conf.contains("backlight = 30s")), "{:?}", image.file(SETTINGS_CONF));

    let restarted = image.restart();
    restarted.click_wheel();
    assert_eq!(restarted.screen_line("setting").as_deref(), Some("setting Backlight 30 s"));
}
