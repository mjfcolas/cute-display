---
name: drive
description: Drive the app image on the simulator or the device plugged in on USB: tap, hold and turn its controls, read its screen as a PNG, its lights, its speaker and its clock, set its clock, read and write its SD card. Use to try a change for real, reproduce a bug, or look at a screen.
---

# Driving Cute Display

Everything goes through the app image's maintenance console: `just remote …` on the
device, `just remote-sim …` on the simulator. `just remote --help` lists the commands;
`tools/link/README.md` shows them.

## On the simulator

1. Start one in the background on a copy of the card, so that it may write to it:
   `cp -r sim-sd <scratchpad>/card`, then `timeout 60 just sim <scratchpad>/card [--speed N]`.
   It opens a window on the user's screen, and its console on `target/simulator.sock`;
   give it a few seconds before the first command. Its log is its output.
2. Act, then look:
   - `just remote-sim tap wheel|yellow|long`, `hold 1500 yellow long`, `turn -2`;
   - `just remote-sim screen <scratchpad>/shot.png`, then Read the PNG; the count it
     prints moves on with every refresh, so compare it before and after an action;
   - `lights`, `sound`, `clock`, `clock --set 2026-09-28T06:59:50` (local without a zone).
3. The simulator stops with its `timeout`, or when its window closes.

## On the device

- Cute Display running, no serial monitor open: `just remote …` as above;
  `just sd-ls`, `sd-get`, `sd-put`, `sd-rm` for the card.
- It runs the image last flashed: trying a change means `just fw-build` then
  `just fw-flash`, which replaces the image on the user's clock. Ask first.
- It is the user's clock. Before acting, keep what it holds: `just sd-get
  cute-display/settings.conf` and `cute-display/apps/alarm/alarm.conf`, and a screen.
  Afterwards, put it back through the controls and check both files and the screen.
- Yellow on the alarm clock's screen switches the alarm on or off; yellow in the system
  app only goes back. Turning the wheel on the alarm clock's screen changes nothing kept.
- `clock --set` writes the real RTC until the next daily network time: set it to the
  time it is, unless the user asked otherwise.
- The log, without the reset `just fw-monitor` makes:

  ```sh
  timeout 30 uv run --quiet --project tools/link python -c "
  import time
  from cute_display_link import usb
  link, end = usb.open_link(), time.monotonic() + 20
  while time.monotonic() < end:
      line = link.readline().decode(errors='replace').strip()
      line and print(line)"
  ```
