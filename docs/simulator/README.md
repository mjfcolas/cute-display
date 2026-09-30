# Simulator

The app image on a computer, in a window: `just sim [card] [--speed N]`.

- `--speed N`: everything runs N times faster than the wall, but the network.
- `--console PATH`: the [maintenance console](../maintenance/README.md) on that Unix
  socket instead of USB; `just sim` puts it on `target/simulator.sock`.

## Controls

- **Wheel**: ← and → turn it, or the mouse wheel; a left click presses it.
- **Yellow button**: a right click.
- **Long button**: space.
- **Esc** quits.
- From another terminal, as on the device: `just remote-sim tap yellow`, `hold 1500
  yellow long`, `turn -2`.

The strip under the glass shows the front light and the reading lamp.

## The SD card

- A directory, `sim-sd/` by default: its `cute-display/` holds the conf files the
  [apps](../../README.md#apps) describe, set on Notre-Dame.
- `just backup-sd` then `just sim backup/sd` runs on a copy of the device's card, but
  writes `settings.conf` there.
- A card without `cute-display/wifi.conf` gets one: the Internet is the computer's, and
  the network in the file is ignored.

## What differs from the device

- The glass takes as long to refresh as the real one at `--speed 1`, and flashes on a
  whole refresh, but never ghosts.
- The RTC starts at the computer's time, and runs at the simulator's speed.
- The speaker is a line in the log, with the loudest sample it was given.
- No battery: the app image does not use it.
