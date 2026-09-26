# Hardware test — design

- **The HAL, directly: no domain or UI layer, on purpose**, so a fault it reports is the
  board's, not the app's.
- Its slow devices get threads of their own: `painter`, `chimes`, `survey`; each module
  says why.
