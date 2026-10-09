# Maintenance console: design

- **A second way into the device, next to the UI.** It works on files, on the controls
  and on what the hardware is told to do, not on the domain. Like `hwtest`,
  `maintenance` depends on `hal` only.
- **The remote enters at the same place as the hardware, and the observation reads what
  the hardware is told, at the same place.** A test goes through every layer, as a person would, and never sees the
  domain. What the screen says is what a person reads on it. It is given with the frame.
- **The console answers a control once the image has taken it.** So a test knows that
  its gesture reached the image, and then waits for the screen to say what follows. If
  the image takes nothing, the answer is an error, never a silent `ok`.
- **Always present, also in releases.** Tests run on the same image that people get.
  The remote gives little more access than the buttons already give.
- **How configuration reaches the device.** A file prepared on a computer is copied into
  the image's own directory (`cute-display/` for the app image). The code that needs it
  reads it each time it uses it, so no restart is needed.
