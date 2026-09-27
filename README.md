# cute-display

Firmware for the [Habity bedside clock](https://habity.design/products/bedside-clock):
ESP32-S3, 3.7" e-paper, a wheel and two buttons, a battery-backed RTC, a microSD card, a
speaker, a reading lamp and a front light.

## Images

Both are flashed into `app1`.

- **App**: small apps, one in front at a time.
  - [System](docs/apps/system/README.md)
  - [Alarm clock](docs/apps/alarm/README.md)
  - [Weather](docs/apps/weather/README.md)
  - [Radar](docs/apps/radar/README.md)
  - [Maintenance console](docs/maintenance/README.md)
- **[Hardware test](docs/hwtest/README.md)**

The [simulator](docs/simulator/README.md) runs the app image on a computer.

## Commands

```sh
just test          # host tests
just lint          # clippy, host and ESP32
just preview       # render an app screen to a PNG (screens: just --list)
just sim           # the app image in a window, the SD card in sim-sd/
just fw            # build the app image, flash it into app1, watch the log
just fw hwtest     # the same with the hardware test image
just fw-boot app0  # back to Habity's firmware (factory: as shipped)
just fw-check      # which app boots, and whether the device can take this one
just backup        # dump the whole flash to backup/ (gitignored)
just sd-ls         # list the SD card; also sd-get, sd-put, sd-rm (monitor closed)
just               # every recipe
```

Building and flashing need the `esp` toolchain (`espup`, sourcing `~/export-esp.sh`),
`espflash`, `ldproxy`, `just` and `uv`; `just test` needs `python3`.

## Further

- [`NOTICE.md`](NOTICE.md): license, data sources, not affiliated with Habity.
- [`AGENTS.md`](AGENTS.md): how this project is written.
- [`DESIGN.md`](DESIGN.md): the architecture.
- [`docs/hardware.md`](docs/hardware.md): the board: pin map, panel controller, flash
  layout.
