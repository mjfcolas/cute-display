# Zen

A sound to relax or fall asleep to, for whole hours, every day at a time or whenever
you start it, while the lights dim.

- **Yellow**: start a session now.
- **Long button**: the settings: when sessions start, how long they last, and the sound.
  - **Wheel**: choose a setting; **long button**: set it.
  - **Start**: its hour, then its minutes; turning the hour past 23 or below 0 starts
    none on its own.
  - **Length**: from 1 to 12 hours.
  - **Sound**: the **wheel** goes round the sounds, each playing softly.
  - **Yellow**: while setting one, puts it back; otherwise, back to the stones and the time.
- At its time, a session comes to the front whatever app was there.

## A session

- The sound rises over its first minute and falls over its last five, round and round
  between.
- The reading lamp and the front light start soft and go out over half an hour, or over
  the session when it is shorter.
- **Wheel**: the volume, kept for the next sessions.
- **Yellow**: stop, the sound fading out.
- Nothing else does anything, so that nothing half asleep changes a setting.
- A start time passed while the device was off waits for the next day.
- The alarm ringing during a session takes the speaker; the session goes on silent.

## Sounds

- Brown noise, always there, and the MP3 and WAV files in `cute-display/apps/zen/sounds/`,
  named by their file names without their extension.
- A file it cannot play plays as brown noise.
- The [setup](../../install.md#run-the-setup) offers to copy Habity's sleep sounds there.

## Configuration

- `cute-display/apps/zen/zen.conf`: when sessions start, their length, the sound and the
  volume, written by the device. Without a card, they last until the power goes.
