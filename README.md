# cute-display

Firmware for the [Habity bedside clock](https://habity.design/products/bedside-clock):
ESP32-S3, 3.7" e-paper, a wheel and two buttons, a battery-backed RTC, a microSD card, a
speaker, a reading lamp and a front light.

## Images

Both live in `app1`, next to the untouched stock firmware.

- **App**: small apps, one in front at a time.
  - [System](docs/apps/system/README.md)
  - [Weather](docs/apps/weather/README.md)
  - [Radar](docs/apps/radar/README.md)
  - [Maintenance console](docs/maintenance/README.md)
- **[Hardware test](docs/hwtest/README.md)**

Neither writes to the RTC or NVS. On the SD card, only the app writes, under
`cute-display/`.

## Commands

```sh
just fw          # build the app image, flash it into app1, watch the log
just fw hwtest   # the same with the hardware test
just fw-stock    # back to the stock firmware
just backup      # dump the whole flash to backup/ (gitignored)
just             # every recipe
```

## Further

- [`AGENTS.md`](AGENTS.md): how this project is written.
- [`DESIGN.md`](DESIGN.md): the architecture.
- [`docs/hardware.md`](docs/hardware.md): the board: pin map, panel controller, flash
  layout.
