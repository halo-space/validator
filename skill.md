# 项目开发约束

## 公共命名

- 公共入口统一使用 `halo::validator`。
- 用户导入统一使用 `use halo::validator::prelude::*;`。
- 不使用冗余复合命名：优先使用 `Config`，不使用 `ValidatorConfig`。
- 规则扩展 trait 命名为 `Provider`，不使用 `RuleProvider`。
- `Provider` 放在 `src/rule/provider.rs`，由 `src/rule.rs` 重导出。
- `Validator` 和 `Config` 放在 `src/validator.rs`。
- `rule.rs` 表示扩展接口，`rules.rs` 表示内置规则集合；两者不能混用。
- 内部模块和文件使用 Rust snake_case；外部 DSL 规则名保持既有语义，例如 `base64rawurl`、`containsany` 不因内部重命名而改变。
- Rust 文件名使用小写 snake_case；目录按领域组织，避免把类型名直接作为文件名。
- 新增公共 API 时，优先选择短、稳定、按模块归属可理解的名称。
- 自定义字段关联规则优先使用显式 `field(name = "...", target = "...")` 语法，避免依赖规则名称后缀推断。
- 内置规则按 `format` 和 `network` feature 分组；默认开启，精简构建使用 `--no-default-features`。
- 异步是默认执行模型；公共 API 和内部执行方法不得使用 `sync`、`async` 或拼写变体作为命名后缀。使用 `Rule::check`、`Validator::value`、`Validator::validate` 等简洁名称。

## 架构约束

- `Validator` 负责执行和运行时缓存。
- `Config` 负责规则、Alias 和扩展 Provider 配置。
- `Config::cache(capacity)` 统一配置运行时缓存容量；不要在业务层直接接触内部缓存类型。
- `Rule`、`Value`、`Signature` 和统一执行模型保持在核心层。
- derive 宏只生成字段访问与校验调用，不复制规则执行逻辑。
- Struct、单值和 Schema 校验继续复用同一套规则执行模型。
- 新增扩展点时，先放入所属领域模块，避免继续扩大顶层聚合文件。
- 不通过兼容别名、废弃包装层或双重 API 解决命名问题；先修正模块边界和正式命名。

## 发布前检查

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features`
