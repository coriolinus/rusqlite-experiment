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

### WASM/Browser Interop

1. Wasm-bindgen is perfectly happy to call `&mut self` methods on JS objects.
1. Downloading an unencrypted database requires some support in the SPA, but the implementation is straightforward overall.

### Encryption Compatibility

3. Rusqlite has Cargo features for sqlcipher but not for Sqlite3 Multiple Ciphers (sqlite3-mc).
   - Current approach: sqlite3-mc on WASM, sqlcipher on native
   - **Compatibility**: PRAGMA statements for basic SQL operations appear equivalent between the two
   - **Detection method**: Unencrypted databases start with `b"SQLite format 3\0"` in their first 16 bytes
     - Easy to check on native; challenging on WASM where the VFS abstracts file access
     - The `ffi` crate's `Database::is_encrypted()` handles this correctly on WASM

### IndexedDB VFS

This approach involves embedding sqlite into the compiled wasm program; database access happens in-process and the database is ultimately backed by IndexedDB.

**Conclusion**: Encryption at rest **kind of works** via the IndexedDB VFS.

There's a roundabout path to enabling it: you can't just do `rusqlite::Connection::open(name)` when you create your connection. Instead you have to use `rusqlite::Connection::open_with_flags_and_vfs`, manually specifying the flags and a VFS name comprising a normal vfs name with an encryption prefix. In this case, that's `"multipleciphers-relaxed-idb"`.

See the [demo](#demo) to see this in action.

**Major Caveat**: As of right now, decryption seems to be broken. Everything works as expected when first encrypting a database and operating on the encrypted data, and we can prove that the database is in fact hiding its data. Unfortunately, decryption attempts currently always produce a "wrong passphrase" error.

**Minor Caveat**: Even though we have attempted to explicitly specify that we should use `sqlcipher` compatibility, it appears not to work. Attempts to load the database always produce errors of this form:

```sql
sqlcipher> .schema
2026-02-25 15:03:22.175: ERROR CORE sqlcipher_page_cipher: hmac check failed for pgno=1
2026-02-25 15:03:22.175: ERROR CORE sqlite3Codec: error decrypting page 1 data: 1
2026-02-25 15:03:22.175: ERROR CORE sqlcipher_codec_ctx_set_error 1
```

Happily, we don't have a real use case for ever moving a CC database from one device to another, so this should be fine.

### `sahpool` OPFS VFS

This approach involves embedding sqlite into the wasm program, but then running that on a separate web worker with a facade in place to hide the communication between the two processes. In theory OPFS is likely faster than IndexedDB, but cross-process communication latency likely kills any perf improvements we'd theoretically gain. No benchmarking has been attempted to determine the truth of the matter.

- Works fine unencrypted; should theoretically have better DB perf than the IndexedDB VFS
- A fair amount of serialization/deserialization scaffolding and hassle is necessary at the web worker boundary
- It is possible to manually specify a VFS which can do encryption at rest
  - but right now decryption fails, possible user error
- Working with OPFS in this context is a real pain for development: once a database has been locked, it is a real pain to get it to unlock again, or even to just delete the whole thing. OPFS eliminates many out of context tools like the filesystem which would make it simple to just delete a DB and start over again.

## Demo

1. Run the demo with `make serve-spa` and then open a browser at `localhost:8080`.
1. The database is unencrypted and accessible; you can create a list and some items.
1. Click the "Download Database" button for a local copy of the sqlite DB

- ```sh
  $ hexyl --length 16 todo_app.sqlite
  ┌────────┬─────────────────────────┬─────────────────────────┬────────┬────────┐
  │00000000│ 53 51 4c 69 74 65 20 66 ┊ 6f 72 6d 61 74 20 33 00 │SQLite f┊ormat 30│
  └────────┴─────────────────────────┴─────────────────────────┴────────┴────────┘
  ```
- ```sql
  sqlite> .mode table
  sqlite> select * from todo_lists join todo_items on todo_lists.id = todo_items.list_id;
  +----+-------+---------------------+----+---------+-------------+--------------+---------------------+
  | id | title |     created_at      | id | list_id | description | is_completed |     created_at      |
  +----+-------+---------------------+----+---------+-------------+--------------+---------------------+
  | 1  | asdf  | 2026-02-25 13:54:22 | 1  | 1       | 1           | 0            | 2026-02-25 13:54:24 |
  | 1  | asdf  | 2026-02-25 13:54:22 | 2  | 1       | 2           | 1            | 2026-02-25 13:54:25 |
  | 1  | asdf  | 2026-02-25 13:54:22 | 3  | 1       | 3           | 0            | 2026-02-25 13:54:27 |
  +----+-------+---------------------+----+---------+-------------+--------------+---------------------+
  ```

4. Click the "Set Encryption Key" button to encrypt the DB; remember your passphrase!
1. Click the "Download Database" button for a local copy of the sqlite DB

- ```sh
  $ hexyl --length 16 todo_app.sqlite
  ┌────────┬─────────────────────────┬─────────────────────────┬────────┬────────┐
  │00000000│ 5c f3 5c bb 09 e8 32 00 ┊ a9 b1 7e 4c 98 3b 20 99 │\×\×_×20┊××~L×; ×│
  └────────┴─────────────────────────┴─────────────────────────┴────────┴────────┘
  ```

6. Refresh the page to drop knowledge of the passphrase
