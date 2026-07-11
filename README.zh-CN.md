# validator

`validator` 是面向 Rust 2024 的数据校验库，基于 derive 宏、类型分派、动态
Schema 校验和可扩展规则注册表实现。项目处于早期开发阶段，不保留旧 API 或旧
配置写法。

[English](README.md)

## 环境要求

- Rust edition: `2024`
- 最低 rustc: `1.97`

## 安装

当前项目按 Git 依赖使用，不发布到 crates.io：

```toml
[dependencies]
validator = { git = "https://github.com/halo-space/validator.git" }
```

需要可复现构建时，请固定 `rev`、`tag` 或 `branch`。

## 快速开始

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required, length(min = 3, max = 20))]
    name: String,

    #[validate(omitempty, email)]
    email: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new().validate(&User {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
    })?;
    Ok(())
}
```

## 文档

- [校验指南](docs/guide.zh-CN.md)：derive、嵌套结构体、集合、选择性校验、
  单值和跨字段规则。
- [扩展能力](docs/extensions.zh-CN.md)：alias、自定义规则和自定义值类型。
- [Schema 校验](docs/schema.zh-CN.md)：YAML/JSON Schema 与 serde 数据。
- [国际化](docs/i18n.zh-CN.md)：内置语言和自定义 locale。
- [规则参考](docs/rules.zh-CN.md)：内置规则族和语义。
- [错误模型](docs/errors.zh-CN.md)：校验失败与配置错误。
- [架构说明](docs/architecture.zh-CN.md)：反射边界和当前限制。
- [开发说明](docs/development.zh-CN.md)：检查、基准和可运行示例。

每份主题文档顶部提供英文版本链接。

## 示例

每个示例都可以独立运行：

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

## 范围

核心库不内置 Web/RPC 框架集成。业务代码选择 locale 和响应格式，再把校验错误
交给 i18n 渲染器或自己的 adapter。
