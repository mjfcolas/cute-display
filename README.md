# cute-display

Firmware for the [Habity bedside clock](https://habity.design/products/bedside-clock):
ESP32-S3, 3.7" e-paper, a wheel and two buttons, a battery-backed RTC, a microSD card, a
speaker, a reading lamp and a front light.

- [`AGENTS.md`](AGENTS.md) — how this project is written.
- [`DESIGN.md`](DESIGN.md) — its architecture.
- [`docs/hardware.md`](docs/hardware.md) — the board: pin map, panel controller, flash layout.

## Status

**The app image** is the base of the real firmware: small apps, one in front at a time.
Hold the long button for a second to open the system app: turn the wheel to choose an app
and press to open it, or change a setting (how long the screen stays lit after a touch,
and the reading lamp: off, 10, 30, 50 or 100 %). Settings are kept on the SD card. The other apps are placeholders
for now: Counter, Echo, and Ping.

**The hardware test image** checks the board and shows what each part says: the panel,
the RTC, the temperature, the I2C bus, the SD card, a Wi-Fi scan, USB power, the battery
sense and the free heap. The wheel steers the front light, a wheel press plays a chime,
the yellow button cycles the reading lamp, and the long button swaps in a checkerboard.

Neither writes to the RTC or NVS; on the SD card, only the app writes, and only under
`cute-display/`.
Both live in `app1` next to the untouched stock firmware.

```sh
just fw          # build the app image, flash it into app1, watch the log
just fw hwtest   # the same with the hardware test
just sd-ls       # list the SD card, over the USB cable (monitor closed)
just sd-get cute-display/settings.conf
just sd-put wifi.conf cute-display/wifi.conf
just fw-stock    # back to the stock firmware
just backup      # dump the whole flash to backup/ (gitignored)
```
