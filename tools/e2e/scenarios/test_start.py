from datetime import datetime, timezone
from zoneinfo import ZoneInfo

PARIS = ZoneInfo('Europe/Paris')
A_THURSDAY_MORNING = datetime(2026, 10, 1, 5, 0, tzinfo=timezone.utc)


def test_the_alarm_clock_is_in_front_on_the_date_and_at_the_time_of_the_clock(clock):
    display = clock.start(at=A_THURSDAY_MORNING)
    screen = display.until_front('alarm')
    assert screen.first_value('date') == 'Thursday 1 October'
    assert screen.first_value('alarm') == 'Alarm off'
    display.until_screen(lambda shown: shown.first_value('time') == f'{display.time().astimezone(PARIS):%H:%M}',
                         "the time shown is not the RTC's in Paris")
