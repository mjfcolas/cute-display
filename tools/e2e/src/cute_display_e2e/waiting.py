"""The app image answers in its own time: a control is taken at once, but the glass shows
it once drawn, and a file is written once the change is made."""
import time

POLL_S = 0.05


def until(condition, timeout_s, what_did_not_happen):
    deadline = time.monotonic() + timeout_s
    while not (found := condition()):
        if time.monotonic() > deadline:
            raise AssertionError(f'{what_did_not_happen}, in {timeout_s} s')
        time.sleep(POLL_S)
    return found
