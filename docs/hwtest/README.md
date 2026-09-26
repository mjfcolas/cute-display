# Hardware test

A second image that checks the board and shows what each part says.

```sh
just fw hwtest                  # build, flash into app1, watch the log
just preview-hwtest [pattern]   # render the report page (or the checkerboard) to a PNG
```

- **Reports**: panel, RTC, temperature, I2C bus, SD card, Wi-Fi scan, USB power, battery
  sense, free heap.
- **Wheel**: front light.
- **Wheel press**: a chime.
- **Yellow button**: cycles the reading lamp.
- **Long button**: swaps in a checkerboard.
- Logs the RTC's registers as it starts: `just backup-rtc` keeps them in `backup/`.
