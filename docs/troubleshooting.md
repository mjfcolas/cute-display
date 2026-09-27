# Troubleshooting

Something not here: [open an issue](https://github.com/mjfcolas/cute-display/issues) with
what `cute-display check` prints. Never attach a backup: it holds your Wi-Fi password.

## The installer says

| Message | Why | What to do |
| --- | --- | --- |
| No Habity found on USB | The cable carries power only, or the clock is not plugged in. | Try another cable, another USB port, and no hub. |
| Several devices found | Other ESP32 boards are plugged in. | Unplug them, or name the clock's port in `CUTE_DISPLAY_PORT`. |
| The port is in use | Another program holds the clock's port: a serial monitor, an Arduino-like tool. | Close it and try again. |
| This user may not open the port | Linux only: the port belongs to a group you are not in. | Join it, then log in again; see [what you need](install.md#what-you-need). |
| The device stopped answering | The connection dropped. An install cut short leaves the clock on Habity's firmware, pointed at Cute Display only once it is written whole; a backup cut short leaves no file. | Unplug the clock, plug it back, run the same command again. |
| no answer from the device, during `setup` | Cute Display is not running, or another program holds the port. Some files may be written and others not. | `cute-display check` says which firmware boots; then `cute-display setup` again. |
| is this computer online? / GitHub: no release yet | The latest release could not be downloaded. | Check the connection, and try again later. |
| Both slots hold Habity's firmware: … Go on? | Habity's firmware updated itself twice: no slot is free. | `y` erases the older of the two; the newer one and the one the clock was shipped with stay. |
| Cannot install: … which one is older cannot be told | Both slots hold Habity's firmware, and a version does not read as numbers. Nothing was written. | Open an issue. |
| Cannot install: no Habity firmware would be left | There would be no way back. Nothing was written. | Open an issue. |
| Cannot install: the partition table is not the one this tool knows | This clock is not like the ones Cute Display was made on. Nothing was written. | Open an issue. |
| Cannot install: secure boot or flash encryption is on | The clock only starts firmware signed by Habity. Nothing was written. | Nothing: Cute Display cannot run on it. |
| Cannot boot cute-display: no cute-display to start here | Cute Display is not installed, or Habity's firmware updated itself over it. | `cute-display install`. |
| The setup assistant does not show up (Windows) | It was run from PowerShell, where its screens do not start. | Run it from the Command Prompt (`cmd`). |

## The clock

| What you see | Why | What to do |
| --- | --- | --- |
| No weather, no time from the Internet | The clock joins 2.4 GHz Wi-Fi only, with the name and password it was given. | Check them with `cute-display setup`, then update the [weather](apps/weather/README.md). |
| The time is off by hours | The time zone. | `cute-display setup`. |
| The radar shows no airports, or old ones | It reads them when it starts and when its place changes ([radar](apps/radar/README.md)). | Restart the clock: `cute-display check` does. |
| It restarts in a loop, or shows nothing | A firmware that does not start. The installer talks to the chip's own loader, which runs before any firmware. | Plug it in: `cute-display boot habity` or `cute-display install` should still work. |

## Putting a backup back

The file `cute-display backup` made is the clock's whole memory. Writing it back, a few
minutes, puts the clock exactly as it was then, with [esptool](https://docs.espressif.com/projects/esptool/),
the tool the installer is built on:

```sh
uvx --from "esptool>=5.1,<6" esptool --chip esp32s3 write-flash 0x0 habity-flash-20260927-101500.bin
```

with the name of your file. Anything installed or set up since is gone; the SD card,
which the backup does not hold, keeps its files.
