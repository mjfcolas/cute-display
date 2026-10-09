# Zen

Plays a sound to relax or fall asleep to, for one hour or more, while the lights dim. A
session starts every day at a set time, or whenever you start it.

- **Yellow**: start a session now.
- **Long button**: the settings: when sessions start, how long they last, and the sound.
  - **Wheel**: choose a setting. **Long button**: set it.
  - **Start**: its hour, then its minutes. Turning the hour above 23 or below 0 means
    that no session starts by itself.
  - **Length**: from 1 to 12 hours.
  - **Sound**: turn the **wheel** to go through the sounds. Each one plays softly.
  - **Yellow**: while you set a value, it cancels the change. Otherwise, it goes back to
    the stones and the time.
- At its start time, a session comes on screen, whatever app was there.

## A session

- The sound gets louder during the first minute, and softer during the last five
  minutes. In between, it plays again and again.
- The reading lamp and the front light start soft, and dim until they are off after half an
  hour, or at the end of the session if it is shorter.
- **Wheel**: the volume, kept for the next sessions.
- **Yellow**: stop. The sound fades out.
- No other control works, so you cannot change a setting by mistake while you are half
  asleep.
- If a start time passes while the device is off, the session waits for the next day.
- If the alarm rings during a session, the alarm takes the speaker, and the session
  continues without sound.

## Sounds

- Brown noise, which is always there, and the MP3 and WAV files in
  `cute-display/apps/zen/sounds/`. Each one is named by its file name without the
  extension.
- If a file cannot be played, brown noise plays instead.
- The [setup](../../install.md#run-the-setup) offers to copy Habity's sleep sounds there.

## Configuration

- `cute-display/apps/zen/zen.conf`: when sessions start, their length, the sound and the
  volume. The device writes it. Without a card, they are kept only until the power goes
  off.
