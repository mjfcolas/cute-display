# Maintenance console — design

- **A second way into the device, beside the UI**, working on files rather than on the
  domain: `maintenance` depends on `hal` alone, like `hwtest`.
- **How configuration reaches the device**: a file prepared on a computer is dropped in
  `cute-display/` and read by whoever needs it, at every use, so it needs no restart.
