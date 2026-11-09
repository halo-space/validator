# Release Checklist

本文记录 crates.io 发布前的检查清单。

## 当前结论

- `validator` 和 `validator-derive` 均允许发布到 crates.io。
- 两个包保持版本一致。
- package name 保持 `validator` 和 `validator-derive`。
- 两个包均包含 `LICENSE-MIT` / `LICENSE-APACHE`。

## crates.io 用法

```toml
[dependencies]
validator = "0.1"
```

业务项目需要可复现构建时，应固定具体版本并提交 `Cargo.lock`。

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

根包在 derive 已发布并进入 crates.io 索引后：

```sh
cargo package --allow-dirty
```

derive 尚未进入索引的本地准备阶段，可以先检查打包内容：

```sh
cargo package --allow-dirty --no-verify
```

根包依赖 crates.io 上的 `validator-derive`，因此正式发布顺序必须是：

1. 发布 `validator-derive`。
2. 等待 crates.io 索引可见后，执行根包的完整 `cargo package` 验证。
3. 发布 `validator`。

## 后续可选收口

- 如果后续走私有 registry，确认 registry 名称、登录 token、发布权限和索引同步延迟。
