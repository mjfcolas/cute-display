# Maintenance console

The SD card does not come out of the case: the app serves it on the USB cable.

```sh
just sd-ls [dir]                               # list
just sd-get cute-display/settings.conf [dest]  # copy off the card
just sd-put wifi.conf cute-display/wifi.conf   # copy onto the card
just sd-rm cute-display/old.conf               # remove
just backup-sd                                 # the whole card into backup/sd/
```

- Close the serial monitor first: both use the same port.
- Everything can be read; only `cute-display/` can be written or removed.
