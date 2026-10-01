from datetime import datetime, timezone

from cute_display_e2e.waiting import until

ALARM_CONF = 'cute-display/apps/alarm/alarm.conf'
# 06:29 in Paris.
A_THURSDAY_MORNING = datetime(2026, 10, 1, 4, 29, tzinfo=timezone.utc)


def test_a_wake_up_time_set_on_the_clock_is_kept_on_the_card(clock):
    display = clock.start(at=A_THURSDAY_MORNING)
    display.until_front('alarm')
    display.tap('long')
    display.until_chosen('row Thursday off')
    display.tap('long')
    display.until_chosen('row Thursday [07]:00')
    display.turn(-1)
    display.until_chosen('row Thursday [06]:00')
    display.tap('long')
    display.turn(30)
    display.until_chosen('row Thursday 06:[30]')
    display.tap('long')
    display.until_chosen('row Thursday 06:30')
    display.tap('yellow')
    display.until_screen(lambda screen: screen.first_value('page') == 'clock', 'the clock did not come back')
    display.tap('yellow')
    display.until_screen(lambda screen: screen.first_value('alarm') == 'Alarm today at 06:30', 'the alarm did not switch on')

    alarm_conf = display.read_file(ALARM_CONF).decode()
    assert 'thursday = 06:30' in alarm_conf and 'enabled = yes' in alarm_conf


def test_the_alarm_rings_in_front_until_snoozed_then_stopped(clock):
    """The sunrise before and the snooze's nine minutes are the integration tier's; here,
    what the real process adds: its speaker."""
    clock.put_before_start(ALARM_CONF, 'enabled = yes\nthursday = 06:30\n')
    display = clock.start(at=A_THURSDAY_MORNING)
    display.until_front('alarm')
    display.tap('wheel')
    display.until_front('system')

    display.until_screen(lambda screen: screen.first_value('alarm') == 'Good morning!', 'the alarm did not ring')
    assert display.screen().front() == 'alarm'
    assert display.is_playing()

    display.tap('long')
    display.until_screen(lambda screen: screen.first_value('alarm') == 'Snoozing until 06:39', 'the alarm did not snooze')
    assert not display.is_playing()

    display.hold(1200, 'yellow', 'long')
    display.until_screen(lambda screen: screen.first_value('alarm') == 'Alarm Thursday at 06:30', 'the alarm did not stop')
    until(lambda: not display.is_playing(), 5, 'the speaker went on playing')
