"""The sessions' times, fades and lights are the integration tier's; here, what the real
process adds: a WAV file of the card, played by its speaker."""
import math
import struct

from cute_display_e2e.waiting import until

GENERAL_CONF = 'cute-display/general.conf'
ZEN_CONF = 'cute-display/apps/zen/zen.conf'
RAIN = 'cute-display/apps/zen/sounds/Rain.wav'
RATE = 44100


def a_second_of_stereo_tone():
    frames = b''.join(struct.pack('<hh', sample, sample)
                      for sample in (int(8000 * math.sin(2 * math.pi * 220 * n / RATE)) for n in range(RATE)))
    fmt = struct.pack('<HHIIHH', 1, 2, RATE, RATE * 4, 4, 16)
    chunks = b'fmt ' + struct.pack('<I', len(fmt)) + fmt + b'data' + struct.pack('<I', len(frames)) + frames
    return b'RIFF' + struct.pack('<I', 4 + len(chunks)) + b'WAVE' + chunks


def test_a_session_started_plays_a_wav_of_the_card_until_stopped(clock):
    clock.put_before_start(RAIN, a_second_of_stereo_tone())
    clock.put_before_start(ZEN_CONF, 'sound = Rain.wav\n')
    clock.put_before_start(GENERAL_CONF, 'time_zone = CET-1CEST,M3.5.0,M10.5.0/3\napps = zen\n')
    display = clock.start()
    display.until_front('zen')
    display.until_screen(lambda screen: screen.first_value('sound') == 'Rain', 'the sound is not Rain')

    display.tap('yellow')
    display.until_screen(lambda screen: screen.first_value('session') == '1 h left', 'no session started')
    until(display.is_playing, 5, 'the speaker did not play')
    display.tap('yellow')
    until(lambda: not display.is_playing(), 5, 'the speaker went on playing')
    assert not [line for line in clock.logs() if 'Rain.wav' in line], 'the WAV was not played'
