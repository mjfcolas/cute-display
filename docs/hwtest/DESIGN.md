# Hardware test: design

- **It uses the HAL directly, with no domain or UI layer, on purpose.** So a fault that
  it reports comes from the board, not from the app.
- Its slow devices get their own threads: `painter`, `chimes`, `survey`. Each module
  says why.
