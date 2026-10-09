# Maintenance console

The SD card cannot be taken out of the case, so the app gives access to it through the
USB cable.

```sh
just sd-ls [dir]                               # list
just sd-get cute-display/settings.conf [dest]  # copy from the card
just sd-put wifi.conf cute-display/wifi.conf   # copy to the card
just sd-rm cute-display/old.conf               # remove
just backup-sd                                 # copy the whole card into backup/sd/
```

- Close the serial monitor first: both use the same port.
- Everything can be read. Only `cute-display/` can be written or removed.
- Tests use the same console. They press the buttons and turn the wheel (the real
  buttons and wheel still work), see what the screen shows and says, see what the lights
  and the speaker do, and read and set the clock: `just remote --help`.
