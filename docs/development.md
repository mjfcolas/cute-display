# Development

- How the project is written: [AGENTS.md](../AGENTS.md).
- Its architecture: [DESIGN.md](../DESIGN.md).
- The board: [hardware.md](hardware.md).

## Images

Both images are flashed into an OTA slot, `app0` or `app1`, next to Habity's firmware.

- **App**: the [apps](../README.md#apps) that people use, and the
  [maintenance console](maintenance/README.md) on the USB cable. The console gives
  access to the SD card, and a remote control for tests.
- **[Hardware test](hwtest/README.md)**: tests every part of the board.

The [simulator](simulator/README.md) runs the app image on a computer. The
[installer](../tools/installer/README.md) is how a computer talks to the device.

## Testing

[The pyramid](testing/README.md): which tier a test goes in, and what each tier proves.

## Apps

- Each app is a crate in [`src/apps/`](../src/apps/). Code that several apps share is a
  crate in [`src/libs/`](../src/libs/). [DESIGN.md](../DESIGN.md#applications) says
  what the engine gives to an app, and what the app gives back.
- The images contain every app of [`src/catalog/`](../src/catalog/). Their Cargo
  features can select fewer apps, or only the system app:
  `cargo run -p simulator --no-default-features --features weather`. On the card,
  [`general.conf`](apps/system/README.md#the-apps) chooses among the apps of the image.
- To add an app:
  - add its crate to the workspace members;
  - add its line and its feature in `catalog`, and the same feature in `simulator` and
    `firmware`;
  - add its name and title in the installer's `config/apps.py` (the test of `catalog`
    says what to write);
  - write its README in `docs/apps/<app>/`.

## Commands

```sh
just test            # host tests
just lint            # clippy, host and ESP32
just preview         # draw an app screen into a PNG (screens: src/simulator/examples/app_screen.rs)
just sim             # the app image in a window, with the SD card in sim-sd/
just fw              # build the app image, flash it, show the log
just fw hwtest       # the same with the hardware test image
just fw e2e          # the same with the test image, which gets its web and time from the stubs
just fw-flash        # flash the last image built (`hwtest` for the hardware test, `e2e` for the test image), no build, no log
just fw-boot habity  # start Habity's firmware again (factory: the firmware the clock was shipped with)
just fw-check        # which app starts, and where Cute Display would go
just backup          # copy the whole flash into backup/ (ignored by git)
just setup           # step by step: back up, install or update, set up (serial monitor closed)
just sd-ls           # list the SD card; also sd-get, sd-put, sd-rm (serial monitor closed)
just remote tap yellow  # the device's controls, lights, speaker, RTC: just remote --help (serial monitor closed)
just remote-sim tap yellow  # the same on the simulator started by `just sim`
just bench           # how fast the card and the console move data, each step measured alone (serial monitor closed)
just e2e simulator   # the end-to-end scenarios on the simulator (testing/README.md)
just e2e device      # the scenarios marked device, on the device: flashes the test image, then the app image again (testing/README.md)
just release         # the app image of a release, in release/ (releasing.md)
just                 # every recipe
```

The installer's tests on the device: `just test-bins <backup>`, `just test-flash <name>`
and `just test-on-device` ([`tools/test_flashes/`](../tools/test_flashes/)).

To update a screenshot of the README: run `just preview <screen>`, then copy
`/tmp/cute-display.png` to `images/<screen>.png`.

Building and flashing need the `esp` toolchain (`espup`, then source `~/export-esp.sh`),
`espflash`, `ldproxy`, `just` and `uv`. [Releasing](releasing.md) says how to make a
release.
