"""The counts, the rounds and the nap's light are the integration tier's; here, what the
real process adds: a countdown over, brought to the front, on its speaker."""
from cute_display_e2e.waiting import until

GENERAL_CONF = 'general.conf'
TIMER_CONF = 'apps/timer/timer.conf'


def test_a_countdown_over_comes_to_the_front_and_rings_until_stopped(clock):
    clock.put_before_start(TIMER_CONF, 'countdown = 1\n')
    clock.put_before_start(GENERAL_CONF, 'time_zone = CET-1CEST,M3.5.0,M10.5.0/3\napps = timer, weather\n')
    display = clock.start()
    display.until_front('timer')
    display.tap('yellow')
    display.until_screen(lambda screen: screen.first_value('status') == 'Counting down', 'the countdown did not start')
    display.tap('wheel')
    display.until_front('system')

    display.until_screen(lambda screen: screen.first_value('status') == 'Time is up', 'the countdown did not end')
    assert display.screen().front() == 'timer'
    until(display.is_playing, 5, 'the speaker did not ring')
    display.tap('yellow')
    display.until_screen(lambda screen: screen.first_value('count') == '1 min', 'the timer did not stop')
    until(lambda: not display.is_playing(), 5, 'the speaker went on ringing')
