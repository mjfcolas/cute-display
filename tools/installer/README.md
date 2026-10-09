# Installer

Installs Cute Display on a Habity bedside clock, sets it up, and can start Habity's
firmware again. It also reads and writes the clock's SD card, puts the radar's airports
on it, and reads the RTC's registers.

- People get it from a release: [installing](../../docs/install.md#in-short).
- From the repository, with [uv](https://docs.astral.sh/uv/): `uv run --project
  tools/installer cute-display …`. The `justfile` does this.
- It finds the device on USB through its [link](../link/README.md).

## Commands

The step-by-step installer:

```sh
cute-display setup
```

Each step as its own command. They work on the device's flash, whatever firmware runs:

```sh
cute-display check                # what each slot contains, which one starts, whether Cute Display can be installed
cute-display backup [directory]   # copy the whole flash into a file; the file contains the Wi-Fi password
cute-display install [image.bin]  # install Cute Display into a slot and start it; without an image, the latest release,
                                  # after showing its UPDATING.md
cute-display boot <target>        # start habity (its newest firmware), factory, cute-display, app0 or app1
```

The SD card, while Cute Display runs and no serial monitor is open:

```sh
cute-display card ls [directory]
cute-display card get <path> [destination]        # to the terminal if there is no destination
cute-display card put <file> <path>               # only under cute-display/
cute-display card rm <path>                       # only under cute-display/
cute-display card pull <directory> <destination>  # "" for the whole card; continues where it stopped
cute-display radar-airports [--at LAT LON]        # the airports around general.conf's place, written to the card; printed with --at
```

While the hardware test image runs:

```sh
cute-display rtc-registers <file>  # save the DS3231's registers, and print them
```

`cute-display <command> --help` gives the details of each command.
