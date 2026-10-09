"""The app image answers at its own speed: it takes a control at once, but the screen shows
the result only once it is drawn, and a file is written only once the change is made."""
import time

POLL_S = 0.05


def until(condition, timeout_s, what_did_not_happen):
    deadline = time.monotonic() + timeout_s
    while not (found := condition()):
        if time.monotonic() > deadline:
            raise AssertionError(f'{what_did_not_happen}, in {timeout_s} s')
        time.sleep(POLL_S)
    return found
