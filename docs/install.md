# Installing Cute Display

Cute Display goes beside Habity's firmware, which stays on the device: going back to it
takes one command. Installing is at your own risk and may void the warranty
([notice](../NOTICE.md)).

## In short

With [what you need](#what-you-need) at hand (on Linux, the right to open the clock's port),
plug the clock into the computer, then paste one line into a terminal:

- Linux and macOS:

  ```sh
  curl -LsSf https://github.com/mjfcolas/cute-display/releases/latest/download/install.sh | sh
  ```

- Windows, in the **Command Prompt** (`cmd`), not PowerShell:

  ```bat
  curl -LsSfo "%TEMP%\cute-display.cmd" https://github.com/mjfcolas/cute-display/releases/latest/download/install.cmd && "%TEMP%\cute-display.cmd"
  ```

It installs [uv](https://docs.astral.sh/uv/) when the computer lacks it, then the latest
installer, then runs [the setup](#run-the-setup). The same line, run again later, updates
everything; `cute-display` is also a command of its own from then on, in a new terminal.

## What you need

- The clock.
- A computer connected to the Internet: tried on Linux and Windows 10 or later; macOS is
  expected to work.
- On Linux, the right to open the clock's port: join the group that owns it, then log in
  again (`sudo usermod -aG dialout $USER`; `uucp` on Arch).

## Run the setup

The line [above](#in-short) runs it; `cute-display setup` runs it again. It goes step by
step, asking before it writes anything:

1. **Back up the clock**: its whole memory, 16 MB, into a file in the current folder, in
   a few minutes; nothing on the clock changes. Should anything ever go wrong, that file
   is the clock exactly as it was, Habity's firmware and its settings included, and
   [writing it back](troubleshooting.md#putting-a-backup-back) restores it.

   > [!WARNING]
   > The backup holds your Wi-Fi password. Keep it somewhere safe and to yourself, and
   > never attach it to an issue.

2. **What the clock holds**, and where Cute Display would go; a clock it cannot go on
   stops there, with nothing written.
3. **Install Cute Display**: the latest release, checked, then the clock restarts on it.
   It goes into a slot Habity's firmware does not use; when both hold one, the question
   says which older one would be erased. The one the clock was shipped with is never
   touched.
4. **Set it up**: your Wi-Fi, your town, your time zone, your apps and the airports on
   the radar, what the clock already holds filled in; written onto the clock at the end,
   which then restarts.

The clock then runs the apps chosen among an [alarm clock](apps/alarm/README.md), the
[weather](apps/weather/README.md) and a [radar](apps/radar/README.md), with the
[system app](apps/system/README.md) a click of the wheel away.

## Update, or change the settings

The line [above](#in-short) again: it takes the latest installer, then the setup offers
the update when a newer release is out, and the settings, which you may leave as they are.

`uv tool uninstall cute-display-installer` removes the installer.

## Go back to Habity's firmware

`cute-display boot habity` restarts the clock on Habity's newest firmware. If that one
does not start, `cute-display boot factory` starts the one the clock was shipped with.

Cute Display stays on the clock: `cute-display boot cute-display` comes back to it,
until Habity's firmware updates itself over it.

Each step also exists on its own, `cute-display backup`, `check`, `install` and `boot`,
for those who prefer them: [the installer's commands](../tools/installer/README.md#commands).

Something else? [Troubleshooting](troubleshooting.md).
