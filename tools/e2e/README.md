# End-to-end scenarios

Scenarios of what a person does with the clock. They play on the simulator, and the
scenarios marked `device` also play on the device. They use only the device's
[console](../../docs/maintenance/README.md): the controls, what the screen says, the
lights, the speaker, the clock and the card. Their place among the tests:
[the pyramid](../../docs/testing/README.md).

- `just e2e simulator`: every scenario, each on its own simulator.
  `just e2e simulator -k alarm` runs only some of them.
- `just e2e simulator --window`: each simulator in a window, so you can watch what a
  scenario does. Do not use the keyboard and the mouse while it runs.
- `just e2e device`: the scenarios marked `device`, [on the device](#on-the-device).
- When a scenario fails, it shows the last thing the screen said, and the log of the
  simulator or the device. A panic in the log also makes it fail.
- A scenario reads the card and the RTC through `clock`. `clock` knows the image's
  directory, and a scenario names files relative to it: `settings.conf`,
  `apps/alarm/alarm.conf`.
- `cards/standard/`: the card that each scenario starts with, as a copy.
- `web/`: the answers of Open-Meteo and adsb.fi, recorded at `AT_THE_RECORDING` in
  `scenarios/conftest.py`. This is also the time when a scenario starts, unless it sets
  its own. The simulator gives these answers. On the device, the stubs give them.

## On the device

The test image (`e2e`) is the app image, using `cute-display-e2e/` on the card. It asks
stubs on this computer for the web and the time. Over HTTPS, it trusts only a test
authority.

- Before you start: the device is plugged in and on the same network as this computer,
  and no serial monitor is open. The device can reach this computer on 8443/tcp and
  8123/udp.
- `just e2e device [pytest arguments]` builds the test image and the app image, flashes
  the test image, and plays the scenarios. Then it flashes the app image again, whether
  the scenarios passed, failed or were stopped.
- `uv run --project tools/e2e pytest tools/e2e/scenarios --device [pytest arguments]`
  only plays the scenarios, on a device that already runs the test image
  (`just fw e2e`).
- `--stubs-address`: use it when the device reaches this computer at a different address
  than the one this computer sends from.
- Before each scenario, the harness empties `cute-display-e2e/`, copies the clock's
  `cute-display/wifi.conf` into it, copies `cards/standard/` into it, sets the RTC and
  the stubs to the scenario's time, and restarts the device. `cute-display/` is not
  changed.
- If the device runs the app image, the harness stops at once and changes nothing: the
  app image's console writes only in `cute-display/`.
- After the run, the RTC keeps the last scenario's time until the app image sets it from
  the Internet, within one minute.
- A scenario is marked `@pytest.mark.device` if it runs in real time. The simulator runs
  ten times faster.
- By hand: `python -m cute_display_e2e.stub_web_server` starts the stubs, and says what
  to put in `cute-display-e2e/e2e.conf`.
- `cute_display_e2e.authority` creates the test authority, `device/ca/`, once. If you
  create another one, build and flash the test image again.

## Recording the web again

The forecast and the aircraft must match the time when the scenarios start, on the
simulator and on the device. Record on a copy of the card, because the simulator writes
to it:

```sh
cp -r tools/e2e/cards/standard /tmp/e2e-card
just sim /tmp/e2e-card --time $(date +%s) --record tools/e2e/web
```

- Open the weather and the radar, then quit.
- Set `AT_THE_RECORDING` to that time, and update the weather and radar scenarios with
  the lines that the screen now says.
