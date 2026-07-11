# Development

[中文](development.zh-CN.md)

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo test --all-targets --all-features
cargo bench --bench validation
cargo package --manifest-path derive/Cargo.toml --allow-dirty
```

The benchmark suite measures derive success and failure, selective validation,
warm and cold direct expressions, warm and cold Schema execution,
`validate_serde`, collection dive, and compound unique projection. Use
`cargo bench --bench validation -- --quick` for a short local verification run.

The current distribution path is Git dependency usage. Registry publishing is
out of scope for now. The root `validator` crate is intentionally not packaged
for crates.io in this phase; only the technical `validator-derive` package is
checked with `cargo package`.

## Examples

```sh
cargo run --example basic
cargo run --example value
cargo run --example collections
cargo run --example selective
cargo run --example custom_rule
cargo run --example custom_value
cargo run --example struct_check
cargo run --example schema
cargo run --example i18n
```

Each example is self-contained and focuses on one validation workflow.
