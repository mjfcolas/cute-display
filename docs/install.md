# Installing Cute Display

Cute Display goes beside Habity's firmware, which stays on the device: going back to it
takes one command. Installing is at your own risk and may void the warranty
([notice](../NOTICE.md)).

## In short

One after the other, in a terminal (on Windows, the Command Prompt):

```sh
# 1. uv, once, then open a new terminal
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"   # Windows
curl -LsSf https://astral.sh/uv/install.sh | sh                                       # macOS, Linux

# 2. the installer: the line under "Installer" on the latest release, such as
uv tool install https://github.com/mjfcolas/cute-display/releases/download/v2026.9.0/cute_display_installer-2026.9.0-py3-none-any.whl

# 3. the clock plugged in: back it up, install Cute Display, set it up, step by step
cute-display setup
```

The sections below say what each step does, and what to check first.

## What you need

- The clock.
- A computer connected to the Internet: tried on Linux and Windows 10 or later; macOS is
  expected to work.
- A terminal to type the commands in: on Windows, the **Command Prompt** (`cmd`), not
  PowerShell, where the setup does not show up.
- [uv](https://docs.astral.sh/uv/getting-started/installation/), which runs the installer,
  installed with the first command [above](#in-short); a terminal opened before it does
  not know it yet.
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

## Run the setup

Plug the clock into the computer, then:

```sh
cute-display setup
```

It goes step by step, asking before it writes anything:

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
4. **Set it up**: your Wi-Fi, your town, your time zone and the airports on the radar,
   what the clock already holds filled in; written onto the clock at the end.

The clock is then an [alarm clock](apps/alarm/README.md) with the
[weather](apps/weather/README.md), a [radar](apps/radar/README.md), and a
[system app](apps/system/README.md) a click of the wheel away.

## Update, or change the settings

1. Each release comes with its installer: run the release's line from
   [get the installer](#get-the-installer) again.
2. `cute-display setup`: it offers the update when a newer release is out, then the
   settings, which you may leave as they are.

`uv tool uninstall cute-display-installer` removes the installer.

## Go back to Habity's firmware

`cute-display boot habity` restarts the clock on Habity's newest firmware. If that one
does not start, `cute-display boot factory` starts the one the clock was shipped with.

Cute Display stays on the clock: `cute-display boot cute-display` comes back to it,
until Habity's firmware updates itself over it.

Each step also exists on its own, `cute-display backup`, `check`, `install` and `boot`,
for those who prefer them: [the installer's commands](../tools/installer/README.md#commands).

Something else? [Troubleshooting](troubleshooting.md).
