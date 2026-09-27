# Installing Cute Display

Cute Display goes beside Habity's firmware, which stays on the device: going back to it
takes one command. Installing is at your own risk and may void the warranty
([notice](../NOTICE.md)).

## What you need

- The clock.
- A computer connected to the Internet: tried on Linux; Windows 10 or later and macOS
  are expected to work.
- A terminal to type the commands in: on Windows, the **Command Prompt** (`cmd`), not
  PowerShell, where the setup assistant does not show up.
- [uv](https://docs.astral.sh/uv/getting-started/installation/), which runs the installer:
  - Windows, in the Command Prompt: `powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"`
  - macOS and Linux, in a terminal: `curl -LsSf https://astral.sh/uv/install.sh | sh`
- On Linux, the right to open the clock's port: join the group that owns it, then log in
  again (`sudo usermod -aG dialout $USER`; `uucp` on Arch).

## Get the installer

On the [latest release](https://github.com/mjfcolas/cute-display/releases/latest), copy
the line under **Installer** and run it in a terminal. It looks like:

```sh
uv tool install https://github.com/mjfcolas/cute-display/releases/download/v2026.9.0/cute_display_installer-2026.9.0-py3-none-any.whl
```

`cute-display` is then a command of its own. If the terminal does not know it, run
`uv tool update-shell` and open a new one.

## Before anything: back up the clock

Plug the clock into the computer, then:

```sh
cute-display backup
```

> [!WARNING]
> The backup holds your Wi-Fi password. Keep it somewhere safe and to yourself, and
> never attach it to an issue.

It copies the clock's whole memory, 16 MB, into a file in the current folder, in a few
minutes; nothing on the clock changes. Should anything ever go wrong, that file is the
clock exactly as it was, Habity's firmware and its settings included, and
[writing it back](troubleshooting.md#putting-a-backup-back) restores it.

## Install

1. `cute-display check`: what the clock holds, and where Cute Display would go. Nothing
   is written.
2. `cute-display install`: the latest Cute Display, checked, then the clock restarts on
   it. It goes into a slot Habity's firmware does not use; when both hold one, it asks
   before erasing the older of the two. The one the clock was shipped with is never
   touched.
3. `cute-display setup`: an assistant asks for your Wi-Fi, your town, your time zone and
   the airports on the radar, then writes them onto the clock. Run it again to change
   any of them.

The clock is then an [alarm clock](apps/alarm/README.md) with the
[weather](apps/weather/README.md), a [radar](apps/radar/README.md), and a
[system app](apps/system/README.md) a click of the wheel away.

## Update

1. Each release comes with its installer: run the release's line from
   [get the installer](#get-the-installer) again.
2. `cute-display install`: the latest release, in place of the one before. The settings
   stay.

`uv tool uninstall cute-display-installer` removes the installer.

## Go back to Habity's firmware

`cute-display boot habity` restarts the clock on Habity's newest firmware. If that one
does not start, `cute-display boot factory` starts the one the clock was shipped with.

Cute Display stays on the clock: `cute-display boot cute-display` comes back to it,
until Habity's firmware updates itself over it.

Something else? [Troubleshooting](troubleshooting.md).
