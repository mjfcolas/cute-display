# Hardware test — design

- **The HAL, directly: no domain or UI layer, on purpose**, so a fault it reports is the
  board's, not the app's.
- `src/firmware/src/bin/hwtest.rs` builds the board and hands its devices to
  `hwtest::bench`.
