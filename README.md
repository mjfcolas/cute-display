# cute-display

Firmware for the [Habity bedside clock](https://habity.design/products/bedside-clock):
ESP32-S3, 3.7" e-paper, a wheel and two buttons, a battery-backed RTC, a microSD card, a
speaker, a reading lamp and a front light.

- [`AGENTS.md`](AGENTS.md) — how this project is written.
- [`DESIGN.md`](DESIGN.md) — its architecture.
- [`docs/hardware.md`](docs/hardware.md) — the board: pin map, panel controller, flash layout.

## Status: hardware test

The only image so far checks the board and shows what each part says:

| Control       | Exercises                                   |
| ------------- | ------------------------------------------- |
| Wheel         | the front light, 10 % per detent            |
| Wheel press   | the speaker, with a chime                   |
| Yellow button | the reading lamp: off, 10, 50, 100 %        |
| Long button   | the panel, swapping the report for a checkerboard |

The report, refreshed every second, covers the panel, the RTC (time, oscillator, alarm line), the
temperature, the I2C bus, the SD card, a Wi-Fi scan, USB power, the battery sense and the free
heap. A filled square passes, a crossed one fails, a hollow one has not been exercised yet.

It writes nothing to the RTC, the SD card or NVS, and lives in `app1` next to the untouched
stock firmware.

```sh
just fw          # build, flash into app1, watch the log
just fw-stock    # back to the stock firmware
just backup      # dump the whole flash to backup/ (gitignored)
```
