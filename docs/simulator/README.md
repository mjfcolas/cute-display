# Simulator

Runs the app image on a computer, in a window: `just sim [card] [flags]`.

- `--speed N`: everything except the network runs N times faster than real time.
- `--console PATH`: the [maintenance console](../maintenance/README.md) on this Unix
  socket instead of USB. `just sim` puts it on `target/simulator.sock`.
- `--headless`: no window. Only the console can use the controls.
- `--time UNIX_SECONDS`: the time at the start. NTP gives this time, and the clock runs
  from there.
- `--web DIR`: HTTP requests get the responses recorded in `DIR`, and any other URL
  cannot be reached. `--record DIR` records the responses from the Internet into `DIR`.
  `--offline`: no network at all.

## Controls

- **Wheel**: ← and → or the mouse wheel turn it. A left click presses it.
- **Yellow button**: a right click.
- **Long button**: space.
- **Esc** quits.
- From another terminal, as on the device: `just remote-sim tap yellow`, `hold 1500
  yellow long`, `turn -2`.

The strip under the screen shows the front light and the reading lamp.

## The SD card

- The card is a directory, `sim-sd/` by default. Its `cute-display/` directory holds the
  conf files that the [apps](../../README.md#apps) describe, with Notre-Dame as the
  place.
- `just backup-sd` then `just sim backup/sd` runs on a copy of the device's card. The
  simulator writes `settings.conf` there.
- If the card has no `cute-display/wifi.conf`, the simulator creates one. The simulator
  uses the computer's Internet connection, and ignores the network in the file.

## Differences from the device

- At `--speed 1`, the screen takes as long to refresh as the real one, and flashes on a
  full refresh. But it never shows ghosting.
- The RTC starts at the computer's time, and runs at the simulator's speed.
- The speaker is a line in the log, with the loudest sample it received.
- No battery: the app image does not use it.
