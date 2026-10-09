# Hardware test

A second image. It tests the board and shows what each part reports.

```sh
just fw hwtest                  # build, flash, show the log
just preview-hwtest [pattern]   # draw the report page (or the checkerboard) into a PNG
```

- **Reports**: panel, RTC, temperature, I2C bus, SD card, Wi-Fi scan, USB power, battery
  sense, free heap.
- **Wheel**: front light.
- **Wheel press**: plays a chime.
- **Yellow button**: the reading lamp's next level.
- **Long button**: switches between the report and a checkerboard.
- At start, it writes the RTC's registers to the log. `just backup-rtc` saves them in
  `backup/`.
