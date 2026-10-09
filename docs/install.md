# Installing Cute Display

Cute Display is installed next to Habity's firmware, which stays on the device. One
command takes you back to it. You install Cute Display at your own risk, and it may void
the warranty ([notice](../NOTICE.md)).

## In short

Check that you have [what you need](#what-you-need) (on Linux, the right to open the
clock's port). Plug the clock into the computer, then paste one line into a terminal:

- Linux and macOS:

  ```sh
  curl -LsSf https://github.com/mjfcolas/cute-display/releases/latest/download/install.sh | sh
  ```

- Windows, in the **Command Prompt** (`cmd`), not in PowerShell:

  ```bat
  curl -LsSfo "%TEMP%\cute-display.cmd" https://github.com/mjfcolas/cute-display/releases/latest/download/install.cmd && "%TEMP%\cute-display.cmd"
  ```

This line installs [uv](https://docs.astral.sh/uv/) if the computer does not have it,
then the latest installer, then runs [the setup](#run-the-setup). Run the same line
again later to update everything. After the first run, `cute-display` is also a command
that you can use in a new terminal.

## What you need

- The clock.
- A computer connected to the Internet. It was tested on Linux and on Windows 10 or
  later. macOS should work.
- On Linux, the right to open the clock's port: join the group that owns the port, then
  log in again (`sudo usermod -aG dialout $USER`; the group is `uucp` on Arch).

## Run the setup

The line [above](#in-short) runs it. `cute-display setup` runs it again. It goes step by
step, and asks before it writes anything:

1. **Back up the clock**: it copies the clock's whole memory, 16 MB, into a file in the
   current folder. This takes a few minutes, and nothing on the clock changes. If
   anything goes wrong later, this file is the clock exactly as it was, with Habity's
   firmware and its settings. [Write it back](troubleshooting.md#putting-a-backup-back)
   to restore the clock.

   > [!WARNING]
   > The backup contains your Wi-Fi password. Keep it in a safe place, do not share it,
   > and never attach it to an issue.

2. **What the clock contains**, and where Cute Display would go. If Cute Display cannot
   go on this clock, the setup stops here, and nothing is written.
3. **Install Cute Display**: the latest release is downloaded and checked, then the
   clock restarts on it. It goes into a slot that Habity's firmware does not use. If
   both slots hold Habity's firmware, the question says which one, the older, would be
   erased.
   The firmware that the clock was shipped with is never touched.
4. **Set it up**: your Wi-Fi, your town, your time zone, your apps and what they need.
   Each question is filled in with what the clock already has. At the end, the setup writes them
   to the clock, which then restarts.

The clock then runs the [apps](../README.md#apps) you chose. The
[system app](apps/system/README.md) is one click of the wheel away.

## Update, or change the settings

Run the line [above](#in-short) again. It gets the latest installer. Then, if a newer
release is out, the setup offers the update, and shows what to
[know before updating](../UPDATING.md), if there is anything. It also shows the
settings, which you can keep as they are.

`uv tool uninstall cute-display-installer` removes the installer.

## Go back to Habity's firmware

`cute-display boot habity` restarts the clock on Habity's newest firmware. If that
firmware does not start, `cute-display boot factory` starts the firmware that the clock
was shipped with.

Cute Display stays on the clock: `cute-display boot cute-display` starts it again. This
works until Habity's firmware updates itself and writes over it.

Each step also exists as its own command: `cute-display backup`, `check`, `install` and
`boot`. See [the installer's commands](../tools/installer/README.md#commands).

Another problem? See [Troubleshooting](troubleshooting.md).
