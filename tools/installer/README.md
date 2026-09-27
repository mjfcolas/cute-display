# Installer

Installs Cute Display on a Habity bedside clock, sets it up, and boots Habity's firmware
again; reads and writes its SD card, puts the radar's airports on it, and reads the RTC's
registers.

- People get it from a release: [installing](../../docs/install.md#get-the-installer).
- From the repository, with [uv](https://docs.astral.sh/uv/): `uv run --project
  tools/installer cute-display …`, which is what the `justfile` does.
- The device is found by its USB IDs (the ESP32-S3's own USB port); `CUTE_DISPLAY_PORT`
  overrides it.

## Commands

The device's flash, whatever runs:

```sh
cute-display check                # what each slot holds, which one boots, whether Cute Display can go in
cute-display backup [directory]   # the whole flash into a file; it holds the Wi-Fi password
cute-display install [image.bin]  # Cute Display into a slot, and boot it; the latest release's without an image
cute-display boot <target>        # start habity (its newest firmware), factory, cute-display, app0 or app1
```

The SD card, Cute Display running and no serial monitor open:

```sh
cute-display setup                                # an assistant: the Wi-Fi, the place, the time zone, the radar's airports
cute-display card ls [directory]
cute-display card get <path> [destination]        # to the terminal without a destination
cute-display card put <file> <path>               # under cute-display/ only
cute-display card rm <path>                       # under cute-display/ only
cute-display card pull <directory> <destination>  # "" for the whole card; resumes where it stopped
cute-display radar-airports [--at LAT LON]        # around radar.conf's place, onto the card; printed with --at
```

The hardware test image running:

```sh
cute-display rtc-registers <file>  # the DS3231's registers, saved and read out
```

`cute-display <command> --help` details each one.
