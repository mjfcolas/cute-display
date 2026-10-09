# Link

The computer's side of the app image's console ([`src/maintenance/`](../../src/maintenance/)).
The [installer](../installer/README.md) uses it for the SD card, and tests use it for the
controls.

- It finds the device by its USB IDs (the ESP32-S3's own USB port). `CUTE_DISPLAY_PORT`
  replaces this search.
- While Cute Display runs and no serial monitor is open:

```sh
just remote tap yellow            # press and release a button: wheel, yellow or long
just remote hold 1500 yellow long # hold buttons down together, then release them
just remote turn -2               # turn the wheel; positive is clockwise
just remote describe              # what the screen says, as text
just remote screen shot.png       # what the screen shows
just remote lights                # also sound, and clock: the RTC's time
just remote clock --set 2026-09-28T06:59:50  # set the RTC, until the next daily update from the network
```

- `just remote-sim …` does the same on the simulator started by `just sim`, through its
  Unix socket.
- `just bench` measures how fast each step goes: a put, the console alone, a card read
  through the console, and a copy on the card. So, when a copy is slow, it shows where
  the time goes. It reads and copies the largest of Habity's ringtones, or else the file
  it put, and removes what it wrote.
