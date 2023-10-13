# Quick-KV

A reliable key-value storage for modern software

## Features

- [x] Binary Based Data-Store
- [x] Serde Supported Data Types
- [x] Thread Safe

## Links

[Documentation] | [Crates.io] | [Github]

## Installation

```bash
cargo add quick-kv
```

## Examples

```rust
use quick_kv::prelude::*;

fn main() -> anyhow::Result<()>
{
    let mut client = QuickClient::<String>::new(None);

    client.get("star this repo")?;

    Ok(())
}
```

[Documentation]: https://docs.rs/quick-kv
[Crates.io]: https://crates.io/crates/quick-kv
[Github]: https://github.com/ThatGuyJamal/quick-kv

## CLI (Beta)

Quick-KV comes with a REPL for interacting with the database.

To install the CLI, run the following command:

```bash
cargo install quick-kv
```

This is different from the `cargo add` command because it installs the CLI globally allowing you to use it as a executable.
