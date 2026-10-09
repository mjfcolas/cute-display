# Changelog

What changes from one release to the next, for people who own the clock.
[Releasing](docs/releasing.md) says how a release is made.

## Unreleased

- [Zen](docs/apps/zen/README.md): plays a sound to relax or fall asleep to, for 1 to 12
  hours, while the lights dim. It starts every day at a set time, or when you start it.
  The sound is brown noise, or one of Habity's sleep sounds, which the setup offers to
  copy.
- Alarm ringtones can also be WAV files.
- [Timer](docs/apps/timer/README.md): a countdown that rings, pomodoro rounds of work
  and breaks with a chime between them, or a nap that wakes you with the light, then
  rings.
- [Breathe](docs/apps/breathe/README.md): breathe to a rhythm (cardiac coherence, 4-7-8
  or square), while the lights and a soft sound rise and fall with you, until you stop.
- In the system app, the list of apps scrolls when it is too long for the screen. An
  arrow shows that there are more apps above or below.
- A change on most of the screen, for example when you go from one app to another, now
  shows in one step, twice as fast.
- The setup copies Habity's sounds about four times faster. The computer reads the SD
  card seven times faster: Habity's card, about 300 MB, takes 15 minutes instead of two
  hours.
- The card now answers the computer's first request. Before, when the clock had been
  idle for some time, the first request often got "no answer from the device".
- The chosen alarm ringtone now plays. Before, the chime often played instead, or the
  same short beep again and again. Files read from or written to the card were
  sometimes wrong for the same reason; this is also fixed.

## 2026.10.0

- Alarm ringtones: Habity's own ringtones, in addition to the chime. You choose one in
  the alarm's settings, and each one plays softly while you turn the wheel. The setup
  offers to copy them from Habity's sounds on the card.
- You can choose which apps run, and in which order: the setup asks.
- A change made while the screen was drawing (for example, the time set by the network)
  sometimes took one minute to show. It now shows at once.
- Before an update, the setup shows what to [know before updating](UPDATING.md). An
  installer that is too old for a release says so, and installs nothing.
- Update with the [one line](docs/install.md#in-short), not with an installer you
  already have: an installer from 2026.9.1 or before does not show what to know, and
  does not write the new settings.

## 2026.9.1

- One line installs the installer and runs the setup, step by step: the backup, Cute
  Display, then its settings. On Windows, it runs in the Command Prompt.

## 2026.9.0

- Apps: [system](docs/apps/system/README.md), [alarm clock](docs/apps/alarm/README.md),
  [weather](docs/apps/weather/README.md), [radar](docs/apps/radar/README.md).
- Access to the SD card through the USB cable: the
  [maintenance console](docs/maintenance/README.md).
- Installed next to Habity's firmware, which can still be started.
