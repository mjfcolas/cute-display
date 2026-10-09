# Troubleshooting

If your problem is not here, [open an issue](https://github.com/mjfcolas/cute-display/issues)
with what `cute-display check` prints. Never attach a backup: it contains your Wi-Fi
password.

## Installer messages

| Message | Why | What to do |
| --- | --- | --- |
| No Habity found on USB | The cable carries only power, or the clock is not plugged in. | Try another cable, another USB port, and no hub. |
| Several devices found | Other ESP32 boards are plugged in. | Unplug them, or put the clock's port in `CUTE_DISPLAY_PORT`. |
| The port is in use | Another program uses the clock's port: a serial monitor, or a tool like Arduino. | Close it and try again. |
| This user may not open the port | Linux only: the port belongs to a group that you are not in. | Join the group, then log in again. See [what you need](install.md#what-you-need). |
| The device stopped answering | The connection was lost. If an install stops before the end, the clock stays on Habity's firmware: it is switched to Cute Display only when Cute Display is fully written. If a backup stops before the end, there is no file. | Unplug the clock, plug it in again, and run the same command again. |
| no answer from the device, while setting up | Cute Display is not running, or another program uses the port. Some files may be written, and others not. | Run `cute-display setup` again. It says which firmware is running, and offers to start Cute Display. |
| is this computer online? / GitHub: no release yet | The latest release could not be downloaded. | Check the connection, and try again later. |
| Install … erasing Habity … there, the older of the two? (setup) / Both slots hold Habity's firmware: … Go on? (install) | Habity's firmware updated itself twice, so no slot is free. | Installing erases the older of the two. The newer one, and the one the clock was shipped with, stay. |
| Cannot install: … which one is older cannot be told | Both slots contain Habity's firmware, and one version is not made of numbers. Nothing was written. | Open an issue. |
| Cannot install: no Habity firmware would be left | There would be no way back. Nothing was written. | Open an issue. |
| Cannot install: the partition table is not the one this tool knows | This clock is different from the ones Cute Display was made on. Nothing was written. | Open an issue. |
| Cannot install: secure boot or flash encryption is on | The clock only starts firmware signed by Habity. Nothing was written. | Nothing: Cute Display cannot run on this clock. |
| Cannot boot cute-display: no cute-display to start here | Cute Display is not installed, or Habity's firmware updated itself and wrote over it. | `cute-display setup` installs it again. |
| The setup does not show up (Windows) | It was run from PowerShell, where its screens do not start. | Run it from the Command Prompt (`cmd`). |

## The clock

| What you see | Why | What to do |
| --- | --- | --- |
| No weather, no time from the Internet | The clock only joins 2.4 GHz Wi-Fi, with the name and password it was given. | Check them with `cute-display setup`, then update the [weather](apps/weather/README.md). |
| The time is wrong by some hours | The time zone is wrong. | `cute-display setup`. |
| The radar shows no airports, or old ones | It reads them when it starts and when its place changes ([radar](apps/radar/README.md)). | Restart the clock: `cute-display check` restarts it. |
| It restarts again and again, or shows nothing | A firmware does not start. The installer talks to the chip's own loader, which runs before any firmware. | Plug it in: `cute-display boot habity` or `cute-display setup` should still work. |

## Putting a backup back

The backup file is the clock's whole memory. Write it back with
[esptool](https://docs.espressif.com/projects/esptool/), the tool that the installer is
built on. It takes a few minutes, and the clock is then exactly as it was when you made
the backup:

```sh
uvx --from "esptool>=5.1,<6" esptool --chip esp32s3 write-flash 0x0 habity-flash-20260927-101500.bin
```

Use the name of your file. Anything installed or set up after the backup is lost. The
SD card is not in the backup, so it keeps its files.
