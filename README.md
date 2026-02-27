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

Rusqlite has Cargo features for sqlcipher but not for Sqlite3 Multiple Ciphers (sqlite3-mc).

- Current approach: sqlite3-mc on WASM, sqlcipher on native
- Unencrypted databases start with `b"SQLite format 3\0"` in their first 16 bytes

Note that working purely on the command line, despite advertising sqlcipher compatibility and using sqlcipher-style encryption,
the two technologies are not actually compatible. So we should not expect to ever be able to use a web CC DB in a non-wasm context.

```sh
$ sqlcipher --version
3.49.2 2025-05-07 10:39:52 17144570b0d96ae63cd6f3edca39e27ebd74925252bbaf6723bcb2f6b486alt1 (64-bit) (SQLCipher 4.9.0 community)
$ sqlite3mc --version
3.51.2 2026-01-09 17:27:48 b270f8339eb13b504d0b2ba154ebca966b7dde08e40c3ed7d559749818cb2075 (64-bit)
$ # the db is encrypted
$ hexyl --length 16 experiment.sqlite
┌────────┬─────────────────────────┬─────────────────────────┬────────┬────────┐
│00000000│ aa 31 44 40 c0 28 98 45 ┊ 26 32 d6 91 8a 5f e3 f6 │×1D@×(×E┊&2×××_××│
└────────┴─────────────────────────┴─────────────────────────┴────────┴────────┘
$ # sqlite3mc can decrypt it
$ sqlite3mc experiment.sqlite
SQLite version 3.51.2 2026-01-09 17:27:48 (SQLite3 Multiple Ciphers 2.2.7)
Enter ".help" for usage hints.
sqlite> pragma cipher='sqlcipher';
sqlcipher
sqlite> pragma key='asdf';
ok
sqlite> select * from sqlite_master limit 0;
sqlite> select title, is_completed, description from todo_lists inner join todo_items on todo_lists.id = todo_items.list_id;
asdf|0|a
asdf|1|s
asdf|0|d
asdf|1|f
sqlite> .quit
$ # sqlcipher cannot decrypt it
$ sqlcipher experiment.sqlite
SQLite version 3.49.2 2025-05-07 10:39:52 (SQLCipher 4.9.0 community)
Enter ".help" for usage hints.
sqlite> pragma cipher='sqlcipher';
sqlite> pragma key='asdf';
ok
sqlite> select * from sqlite_master limit 0;
2026-02-26 17:17:15.769: ERROR CORE sqlcipher_page_cipher: hmac check failed for pgno=1
2026-02-26 17:17:15.769: ERROR CORE sqlite3Codec: error decrypting page 1 data: 1
2026-02-26 17:17:15.769: ERROR CORE sqlcipher_codec_ctx_set_error 1
Parse error: file is not a database (26)
sqlite> .quit
```

### IndexedDB VFS

This approach involves embedding sqlite into the compiled wasm program; database access happens in-process and the database is ultimately backed by IndexedDB.

**Conclusion**: Encryption at rest **kind of works** via the IndexedDB VFS.

There's a roundabout path to enabling it: you can't just do `rusqlite::Connection::open(name)` when you create your connection. Instead you have to use `rusqlite::Connection::open_with_flags_and_vfs`, manually specifying the flags and a VFS name comprising a normal vfs name with an encryption prefix. In this case, that's `"multipleciphers-relaxed-idb"`.

See the [demo](#demo) to see this in action.

**Major Caveat**: As of right now, decryption seems to be broken. Everything works as expected when first encrypting a database and operating on the encrypted data, and we can prove that the database is in fact hiding its data. Unfortunately, decryption attempts currently always produce a "wrong passphrase" error.

There's some lore on the internet that for an existing encrypted database, the only correct way to decrypt it is by
providing the key pragma as the first action on connection. However, after refactoring to ensure that when a database is
encrypted the key pragma is always the first action on connection, we still encounter the same kinds of errors. Encrypting a database
works fine, and performing operations on an encrypted database is fine (including unsetting the key to decrypt the entire DB), but
decrypting the database when it is not already unlocked just fails.

Whatever the problem is, it's not (only) that the phrase hashing is inconsistent or that it's not correctly deducing the encryption scheme.
We've added explicit cipherscheme updates per [the instructions](https://utelle.github.io/SQLite3MultipleCiphers/docs/configuration/config_sql_pragmas/#key-handling).
We also experimented with manually implementing hashing, but removed that because it didn't fix the problem and was just noise in the code.

**Minor Caveat**: sqlite3mc and sqlcipher are [not actually compatible](#encryption-compatibility). Happily, we don't have a real use case for ever moving a CC database from one device to another, so this should be fine. But in any case, this frees us to experiment among other potential encryption schemes to see if any of them work better.

### `sahpool` OPFS VFS

This approach involves embedding sqlite into the wasm program, but then running that on a separate web worker with a facade in place to hide the communication between the two processes. In theory OPFS is likely faster than IndexedDB, but cross-process communication latency likely kills any perf improvements we'd theoretically gain. No benchmarking has been attempted to determine the truth of the matter.

- Works fine unencrypted; should theoretically have better DB perf than the IndexedDB VFS
- Got moderately quickly to the same state as the current IDB-backed implementation, to wit: encrypting a blank DB works, and operating on a freshly-encrypted DB works, but once the DB is locked, establishing a new unencrypted connection tends to fail for mysterious reasons.
- Working with OPFS is a real pain for development: once a database has been locked, it is a real pain to get it to unlock again, or even to just delete the whole thing. OPFS eliminates many out of context tools like the filesystem which would make it simple to just delete a DB and start over again.
- The requirement to communicate via channels and replicate the whole program's interface dramatically increases the maintenance burden, at least for programs of this size. 
- This experiment targets only the `JS -> IPC -> Rust/WASM in the worker` flow. We didn't even attempt `Rust/WASM -> JS -> IPC -> Rust/WASM in the worker`. 
- Most recent commit: [`7e547c4`](https://github.com/coriolinus/rusqlite-experiment/tree/7e547c4d14453cf2900ff24de1925476e799d4c7)

**Conclusion**: unless we are forced into it, the additional latency and overhead of routing all DB work to a web worker through the JS layer is a huge pain to deal with and we're better off avoiding it. 

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
