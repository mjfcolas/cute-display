"""The steps' seconds and the curves are the unit tier's; here, what the real process adds:
its lights and its speaker following the breaths."""
import time

from cute_display_e2e.waiting import until

GENERAL_CONF = 'cute-display/general.conf'
# At the scenarios' speed, a breath of the coherence rhythm, ten seconds, lasts one.
WATCHED_S = 3


def test_started_the_reading_lamp_rises_and_falls_and_a_sound_plays_until_stopped(clock):
    clock.put_before_start(GENERAL_CONF, 'time_zone = CET-1CEST,M3.5.0,M10.5.0/3\napps = breathe\n')
    display = clock.start()
    display.until_front('breathe')
    assert display.lights().reading_lamp_percent == 0

    display.tap('yellow')
    display.until_screen(lambda screen: screen.first_value('status') == 'Breathing', 'breathing did not start')
    until(display.is_playing, 5, 'the speaker did not play')
    seen = set()
    watched_until = time.monotonic() + WATCHED_S
    while time.monotonic() < watched_until:
        seen.add(display.lights().reading_lamp_percent)
    assert max(seen) >= 20 and min(seen) <= 5, f'the reading lamp did not breathe: {sorted(seen)}'

    display.tap('yellow')
    display.until_screen(lambda screen: screen.first_value('status') == 'Until you stop it', 'breathing did not stop')
    until(lambda: not display.is_playing(), 5, 'the speaker went on playing')
    assert display.lights().reading_lamp_percent == 0
