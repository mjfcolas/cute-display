# Development

How the project is written is [AGENTS.md](../AGENTS.md); its architecture,
[DESIGN.md](../DESIGN.md); the board, [hardware.md](hardware.md).

## Images

Both are flashed into an OTA slot, `app0` or `app1`, beside Habity's firmware.

- **App**: the [apps](../README.md#apps) people use, and the
  [maintenance console](maintenance/README.md) on the USB cable: the SD card, and a remote
  for tests.
- **[Hardware test](hwtest/README.md)**: exercises every part of the board.

The [simulator](simulator/README.md) runs the app image on a computer, and the
[installer](../tools/installer/README.md) is how a computer reaches the device.

## Apps

- An app is a crate in [`src/apps/`](../src/apps/), and what several share, one in
  [`src/libs/`](../src/libs/); [DESIGN.md](../DESIGN.md#applications) says what the
  engine lends an app and what it gives back.
- The images hold every app of [`src/catalog/`](../src/catalog/). Their Cargo features
  pick fewer, or none but the system app:
  `cargo run -p simulator --no-default-features --features weather`. On the card,
  [`general.conf`](apps/system/README.md#the-apps) chooses among those of the image.
- A new app: its crate among the workspace's members, its line and its feature in
  `catalog`, the same feature in `simulator` and `firmware`, its name and title in the
  installer's `config/apps.py` (`catalog`'s test says what to write), and its README in
  `docs/apps/<app>/`.

## Commands

```sh
just test            # host tests
just lint            # clippy, host and ESP32
just preview         # render an app screen to a PNG (screens: just --list)
just sim             # the app image in a window, the SD card in sim-sd/
just fw              # build the app image, flash it, watch the log
just fw hwtest       # the same with the hardware test image
just fw-flash        # flash the image last built (`hwtest` for the test one), no build, no log
just fw-boot habity  # back to Habity's firmware (factory: as shipped)
just fw-check        # which app boots, and where Cute Display would go
just backup          # dump the whole flash to backup/ (gitignored)
just setup           # step by step: back up, install or update, set up (monitor closed)
just sd-ls           # list the SD card; also sd-get, sd-put, sd-rm (monitor closed)
just remote tap yellow  # the device's controls: tap, hold, turn (monitor closed)
just remote-sim tap yellow  # the same on `just sim`'s simulator
just release         # the app image of a release, in release/ (releasing.md)
just                 # every recipe
```

The installer's cases on the device: `just test-bins <backup>`, `just test-flash <name>`
and `just test-on-device` ([`tools/test_flashes/`](../tools/test_flashes/)).

The README's screenshots are `just preview alarm`, `weather`, `radar` and `system`, each
`/tmp/cute-display.png` copied into `images/<screen>.png`.

Building and flashing need the `esp` toolchain (`espup`, sourcing `~/export-esp.sh`),
`espflash`, `ldproxy`, `just` and `uv`. [Releasing](releasing.md) says how a release is
made.
