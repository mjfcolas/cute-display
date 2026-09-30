# Link

A computer's end of the app image's console ([`src/maintenance/`](../../src/maintenance/)):
the SD card for the [installer](../installer/README.md), the controls for tests.

- The device is found by its USB IDs (the ESP32-S3's own USB port); `CUTE_DISPLAY_PORT`
  overrides it.
- Cute Display running and no serial monitor open:

```sh
just remote tap yellow            # a button pressed and released: wheel, yellow or long
just remote hold 1500 yellow long # held down together, then released
just remote turn -2               # the wheel, clockwise positive
```

- `just remote-sim …` does the same on the simulator `just sim` started, through its Unix
  socket.
