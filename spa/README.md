# Rusqlite Experiment SPA

A single-page app designed to showcase the use case where:

- A Rust crate embeds Sqlite via Rusqlite and presents a storage interface
- That Rust crate is compiled to wasm
- A JS application uses the Rust code.

## Files

### In this directory

- `index.html`: landing page, pure scaffolding
- `style.css`: basic styling to improve the UI
- `main.ts`: application core

### Imported during bundling

- `ffi.d.ts`: not actually bundled but describes `ffi.js`
- `ffi.js`: contains calls into `ffi_bg.js`
- `ffi_bg.js`: contains calls into `ffi_bg.wasm`, which the Rust code has been compiled into
