# Necromancy

> [!NOTE]
> This part of the repository is not related to the `postgres_sync` implementation
> and serves no practical purpose - it's just a historical record.

The `postgres` crate used to have a synchronous implementation until version `0.15.2`,
but then it [was converted](https://github.com/sfackler/rust-postgres/commit/14571ab0292f5fa11302b39fded69d98ee99b660) into a wrapper around `tokio-postgres`
([#461](https://github.com/rust-postgres/rust-postgres/issues/461)).

Even though crates.io doesn't delete old crates, trying it out is not as easy as running `cargo add postgres@0.15.2`.
This crate depends on a number of cryptography crates that are in the habit of yanking old versions.
Cargo refuses to resolve dependencies to yanked versions unless those specific versions are already mentioned in `Cargo.lock`.
So I had to perform manual surgery on `Cargo.lock` to make it work.
And I didn't want this (pointless) effort to go to waste, so I'm preserving the result here.
