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
and the reading lamp: off, 10, 30, 50 or 100 %). Settings are kept on the SD card.

**Weather**, the first real app, shows today's weather and the week's at one place, from
Open-Meteo: turn the wheel for the week, press it to update. It needs two files on the SD
card, put there with `just sd-put`:

```text
# cute-display/weather.conf
place = Paris
latitude = 48.85
longitude = 2.35

# cute-display/wifi.conf
ssid = MyNetwork
password = secret
```

**Radar** shows the aircraft around a place, from adsb.fi: north up, a triangle per
aircraft pointing where it flies, the five nearest listed with their altitude and distance.
Turn the wheel for the range (5 to 100 km), press it to update. It updates every 15 s while
it is on screen, and needs its own place:

```text
# cute-display/radar.conf
place = Notre-Dame
latitude = 48.8530
longitude = 2.3499
airport_labels = LFPG, LFPO, LFPB
```

Aircraft carry the last two letters of their registration, placed so that no two labels
cover each other. Airports and airfields are dots, and those listed in `airport_labels`
carry their code: `just radar-airports` finds them within 100 km of the radar's place (from
OurAirports) and puts them on the device.

The Wi-Fi is joined when something needs it and left a minute after the last request.

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
