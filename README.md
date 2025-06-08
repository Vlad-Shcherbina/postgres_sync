> [!CAUTION]
> This README is a placeholder, TODO

`postgres_sync` implements a subset of the `postgres` crate API.
It doesn't depend on `tokio` and uses `std::net` for networking instead.

Currently implemented:
* `Client::connect()` with connection strings of the form `"postgres://user:password@host:port/db"`
* `Client::query_raw()`
* `Client::query_one()`
* `Client::batch_execute()`
* `Client::execute()`
* `Client::transaction()`
