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
2. Those calls appear to succeed at the DB level.
3. But the database isn't modified. More investigation needed.