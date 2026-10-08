# End-to-end scenarios

What a person does with the clock, played on the simulator, and those marked `device` on
the device too, through its [console](../../docs/maintenance/README.md) alone: the
controls, what the glass says, the lights, the speaker, the clock and the card. Where
they stand among the tests: [the pyramid](../../docs/testing/README.md).

- `just e2e`: every scenario, each on a simulator of its own; `just e2e -k alarm` for
  some.
- `just e2e --window`: each simulator in its window, to watch what a scenario does; the
  keyboard and the mouse are best left alone meanwhile.
- `just e2e-device`: the scenarios marked `device`, [on the device](#on-the-device).
- A scenario fails on what the glass said last, and the simulator's or the device's log
  comes with it; a panic in the log fails it too.
- A scenario reads the card and the RTC through `clock`, which knows the image's
  directory, and names its files in it: `settings.conf`, `apps/alarm/alarm.conf`.
- `cards/standard/`: the card each starts on, a copy of it.
- `web/`: the answers of Open-Meteo and adsb.fi, recorded at `AT_THE_RECORDING` in
  `scenarios/conftest.py`, the time a scenario starts at unless it gives its own; the
  simulator answers them, and on the device the stubs do.

## On the device

The test image (`e2e`) is the app image in `cute-display-e2e/` on the card, its web and
its time asked of stubs on this computer, over HTTPS it trusts for a test authority
alone.

- Before: the device plugged in, on the same network as this computer, no serial monitor
  open; this computer reachable on 8443/tcp and 8123/udp.
- `just e2e-device [pytest arguments]` builds the test image and the app image, flashes
  the test image, plays the scenarios, then flashes the app image back, whether they
  passed, failed or were stopped.
- `just e2e --device [pytest arguments]` plays them alone, on a device already running the
  test image (`just fw e2e`).
- `--stubs-address` when the device reaches this computer at another address than the
  one it leaves from.
- Each scenario empties `cute-display-e2e/`, copies the clock's `cute-display/wifi.conf`
  there, lays out `cards/standard/`, sets the RTC and the stubs to its time, and restarts
  the device. `cute-display/` is left as it was.
- On the app image, the harness stops at once, having changed nothing: its console
  writes nowhere but `cute-display/`.
- After, the RTC holds the last scenario's time until the app image sets it from the
  Internet, within a minute.
- A scenario is marked `@pytest.mark.device` when it runs in real time: the simulator
  runs ten times faster.
- By hand: `python -m cute_display_e2e.stub_web_server` serves the stubs and says what to
  put in `cute-display-e2e/e2e.conf`.
- The test authority, `device/ca/`, is made once by `cute_display_e2e.authority`; another
  means building and flashing the test image again.

## Recording the web again

The forecast and the aircraft must be about the time the scenarios start at, on the
simulator and on the device. On a copy of the card, which the simulator writes to:

```sh
cp -r tools/e2e/cards/standard /tmp/e2e-card
just sim /tmp/e2e-card --time $(date +%s) --record tools/e2e/web
```

- Open the weather and the radar, then quit.
- `AT_THE_RECORDING` becomes that time, and the weather and radar scenarios expect the
  lines the glass now says.
