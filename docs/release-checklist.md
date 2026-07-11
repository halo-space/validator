# Release Checklist

本文记录 Git 依赖模式下的发布前检查结论。

## 当前结论

- 当前先不发布 crates.io，也不发布私有 registry。
- 当前分发方式是 Git 依赖。
- package name 保持 `validator`。
- 根包 package 清单已经包含 `LICENSE-MIT` / `LICENSE-APACHE`。

## Git 依赖用法

```toml
[dependencies]
validator = { git = "https://github.com/halo-space/validator.git" }
```

业务项目需要可复现构建时，应固定 `rev`、`tag` 或 `branch`。

## 检查命令

```sh
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
for example in basic value collections selective custom_rule custom_value struct_check schema i18n; do cargo run --quiet --example "$example"; done
openspec validate --all --strict
git diff --check
```

derive 包：

```sh
cargo package --manifest-path derive/Cargo.toml --allow-dirty
```

根包 Git 依赖模式下只检查清单：

```sh
cargo package --list --allow-dirty
```

注意：`cargo package --list` 中出现的 `Cargo.toml.orig` 是 Cargo 打包 path dependency 时自动保留的原始 manifest，不是工作区脏文件。

## 后续可选收口

- 如果后续走 crates.io，单独设计 package name，同时尽量保持 Rust 代码导入路径仍是 `validator`。
- 如果后续走私有 registry，确认 registry 名称、登录 token、发布权限和索引同步延迟。
- 如果后续发布 registry，需要先处理 `validator-derive` 与根包的发布顺序。
