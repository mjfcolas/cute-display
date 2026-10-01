from cute_display_e2e.waiting import until

SETTINGS_CONF = 'cute-display/settings.conf'


def open_reading_lamp_setting(display):
    display.until_front('alarm')
    display.tap('wheel')
    display.until_front('system')
    display.turn(4)
    return display.until_screen(lambda screen: (screen.chosen() or '').startswith('setting Reading lamp'), 'the reading lamp not chosen')


def test_the_reading_lamp_set_in_the_system_app_shines_and_is_kept_across_a_restart(clock):
    display = clock.start()
    assert open_reading_lamp_setting(display).chosen() == 'setting Reading lamp off'
    display.tap('long')
    display.until_chosen('setting Reading lamp 10%')
    until(lambda: display.lights().reading_lamp_percent == 10, 5, 'the reading lamp did not shine')
    until(lambda: 'reading_lamp = 10%' in display.read_file(SETTINGS_CONF).decode(), 5, 'the setting is not on the card')

    display = clock.start()
    assert open_reading_lamp_setting(display).chosen() == 'setting Reading lamp 10%'
    until(lambda: display.lights().reading_lamp_percent == 10, 5, 'the reading lamp did not shine again')
