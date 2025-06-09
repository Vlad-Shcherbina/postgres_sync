# postgres_sync

[![crates.io](https://img.shields.io/crates/v/postgres_sync.svg)](https://crates.io/crates/postgres_sync)
[![docs.rs](https://img.shields.io/docsrs/postgres_sync/latest)](https://docs.rs/postgres_sync)
[![CI status](https://github.com/Vlad-Shcherbina/postgres_sync/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Vlad-Shcherbina/postgres_sync/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A purely synchronous PostgreSQL client library that provides the same interface as the `postgres` crate,
but uses standard library networking instead of `tokio`.
This results in a smaller dependency graph and more predictable blocking behavior.

The original `postgres` crate was synchronous until version 0.15.2.
After that, it became a thin wrapper around the async `tokio-postgres` crate.
This project revives the purely synchronous approach for applications where async is not a requirement.

## When to use `postgres_sync`

✅ **Use this library when:**
- Your application doesn't deal with too many concurent connections
- You want to minimize your dependency tree and avoid `tokio`
- You prefer predictable, straightforward blocking I/O

❌ **Don't use this library when:**
- Your application faces the [C10k problem](https://en.wikipedia.org/wiki/C10k_problem)
- Your project already depends on `tokio`
- You require the full, battle-tested API of the official `postgres` crate

## Usage

Replace your `postgres` dependency in `Cargo.toml`:

```toml
# before
[dependencies]
postgres = { version = "...", features = ["with-serde_json-1"] }

# after
[dependencies]
postgres = { package = "postgres_sync", version = "0.1", features = ["with-serde_json-1"] }
```

That's it! It's designed as a drop-in replacement.

## Feature status

`postgres_sync` implements a subset of the `postgres` crate's API.
The goal is for any implemented feature to behave identically to the original.

### Implemented

- `Client::connect()` with connection strings of the form `"postgresql://user:password@host:port/db"` and `NoTls`
- `Client::transaction()`
- `{Client, Transaction}::query_raw()`
- `{Client, Transaction}::query_one()`
- `{Client, Transaction}::query()`
- `{Client, Transaction}::batch_execute()`
- `{Client, Transaction}::execute()`
- `with-serde_json-1` feature flag
- `with-chrono-0_4` feature flag

### Limitations and divergences

`postgres_sync` is not a 1:1 clone of the `postgres` API.
It aims for compatibility where implemented, but provides a partial and evolving implementation.
This means you may encounter differences in the following ways:

- **API surface**: Entire types, traits, or modules from the original crate may be missing.
- **Incomplete features**: An implemented method might not support the full range of parameters as its `postgres` counterpart.
  For example, `Client::connect()` supports connection strings but does not yet handle TLS configuration options.
- **Simplified error handling**: This crate uses its own error types. They are not type-compatible with the errors from the `postgres` crate.

## Project layout

- `postgres_sync` - The main library crate
- `verify_*` - Binary test crates that run the same test suite against a live PostgreSQL instance.
  - `verify_orig` links against the original `postgres` crate.
  - `verify_sync` links against this crate (`postgres_sync`).
  - The source code is shared via a symlink (`verify_sync/src` -> `verify_orig/src`) to guarantee the tests are identical.

## License

This project is licensed under the same terms as the `postgres` crate:
- MIT License
- Apache License 2.0

Choose whichever license works best for your use case.

## Acknowledgments

`postgres_sync` is inspired by and heavily dependent on the excellent [`rust-postgres`](https://github.com/sfackler/rust-postgres) collection of crates by Steven Fackler.
It specifically uses the `postgres-protocol` and `postgres-types` crates for handling low-level details.
