# Maintenance console — design

- **A second way into the device, beside the UI**, working on files, the controls and
  what the hardware is told, rather than on the domain: `maintenance` depends on `hal` alone, like `hwtest`.
- **The remote enters where the hardware does, and the observation watches it there**:
  a test goes through every layer, as a person would, and never sees the domain.
- **Always there, releases included**: tests run on the image people get, and the
  remote opens little more than the buttons already do.
- **How configuration reaches the device**: a file prepared on a computer is dropped in
  `cute-display/` and read by whoever needs it, at every use, so it needs no restart.
