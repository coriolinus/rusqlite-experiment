# Ruqslite Experiment

How far can we get writing a TODO application that works both on the command line and the internet, backed by Rusqlite?

## Running the experiment

### Native

```sh
$ cargo run -p cli -- --help
Usage: todo-list [OPTIONS]

Options:
  -p, --db-path <DB_PATH>
          Path to the database

          [default: $HOME/.local/share/todo-list/db.sqlite]

  -l, --log [<LEVEL>]
          Enable logging

          If this flag is set without an explicit level argument, defaults to "info".

          [possible values: trace, debug, info, warn, error]

  -h, --help
          Print help (see a summary with '-h')
```

### WASM

#### Setup

- `rustup target add wasm32-unknown-unknown`
- install `wasm-bindgen-cli`

#### Build

```sh
make serve-spa
```


## Notes and Findings

1. Wasm-bindgen is perfectly happy to call `&mut self` methods on JS objects.
2. Downloading an unencrypted database takes a little bit of support in the SPA, but isn't unduly complicated overall.
3. Rusqlite has Cargo features for enabling sqlcipher, but not for Sqlite3 Multiple Ciphers.
    - Do we need to add a feature, or are we happy using sqlite3-mc on wasm and sqlcipher on non-wasm?
    - Are they compatible with each other if the keys are known?
    - PRAGMA statements for basic sql operations appear to be equivalent
    - But the recommended way to tell "is this database encrypted" is to look at the first 16 bytes: if they match `b"SQLite format 3\0"`, it's not encrypted; otherwise it is.
        - Easy on native, hard on WASM when that's abstracted behind a VFS we don't have real access to
        - `ffi` crate's `Database::is_encrypted()` does the right thing on wasm
    - TBD: does `rusqlite` delegate eventually down to [`RelaxedIdbUtil::import_db_unchecked`](https://docs.rs/sqlite-wasm-vfs/latest/sqlite_wasm_vfs/relaxed_idb/struct.RelaxedIdbUtil.html#method.import_db_unchecked) instead of [`import_db`](https://docs.rs/sqlite-wasm-vfs/latest/sqlite_wasm_vfs/relaxed_idb/struct.RelaxedIdbUtil.html#method.import_db), which is necessary if the database is encrypted? If not, can we force that somehow?
