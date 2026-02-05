# Turso Experiment

How far can we get writing a TODO application that works both on the command line and the internet, backed by Turso?

## Notes and Findings

1. Migrations are a pain and we may have to write our own migration framework
    - `refinery` has no native support for Turso
    - and bringing in `rusqlite` only for purpose of migrations is silly, and will likely fail once we encrypt the database
    - so we introduce a connection wrapper, whatever
    - but the traits `refinery` uses are only sync, and `turso` is async-first
    - and we can't just `smol::block_on` because `block_on` is illegal in a wasm context
2. Getting a rowset is annoyingly manual
    - [`Rows`](https://docs.rs/turso/0.4.4/turso/struct.Rows.html) has an annoyingly manual interface: just `async fn next`, no iterator or map combinators
    - Can't get a value by column name, only by column index
    - `query_row` returns no error if more than one row is returned
3. Encryption is a planned feature, but it's experimental for now at least.
4. Builder feels underbaked: path argument is `&str` not `impl AsRef<Path>`.
5. We still occasionally encounter TODO items in the library
    - `Parse error: TODO: parenthesized expression with multiple arguments not yet supported` from `RETURNING (id, created_at)`.
        - Though that might have been an sql error anyway
6. Ultimately in the native context it appears to work, at least for simple cases.
    I would not yet call it ready for prime time in this context due to its various rough edges, but there's clear and substantial progress since last we looked at it.
