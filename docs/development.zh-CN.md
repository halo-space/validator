# 开发说明

[English](development.md)

## 开发命令

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo test --all-targets --all-features
cargo bench --bench validation
cargo package --manifest-path derive/Cargo.toml --allow-dirty
```

benchmark 覆盖 derive 成功/失败、选择性校验、direct expression 冷/热路径、
Schema 冷/热路径、`validate_serde`、集合 dive 和 compound unique 投影。
本地快速检查可以运行 `cargo bench --bench validation -- --quick`。

当前分发路径是 Git 依赖使用，暂不处理 registry 发布。根 `validator` crate 这一阶段不按 crates.io package 方式收口；这里只对技术包 `validator-derive` 做 `cargo package` 检查。

## 示例

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

每个示例都可以独立运行，并且只聚焦一种校验场景。
