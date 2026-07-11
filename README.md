# validator

`validator` is a Rust 2024 validation library built around derive macros,
typed value dispatch, dynamic Schema validation, and an extensible rule
registry. It is an early-stage project and does not preserve legacy APIs or
configuration spellings.

[中文文档](README.zh-CN.md)

## Requirements

- Rust edition: `2024`
- Minimum supported rustc: `1.97`

## Install

This project is currently consumed from Git rather than crates.io:

```toml
[dependencies]
validator = { git = "https://github.com/halo-space/validator.git" }
```

Pin a `rev`, `tag`, or `branch` when reproducible builds matter.

## Quick Start

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

## Documentation

- [Validation guide](docs/guide.md): derive, nested structs, collections,
  selective validation, direct values, and cross-field rules.
- [Extensions](docs/extensions.md): aliases, custom rules, and custom values.
- [Schema validation](docs/schema.md): YAML/JSON schemas and serde data.
- [Internationalization](docs/i18n.md): built-in and custom locales.
- [Rule reference](docs/rules.md): built-in rule families and semantics.
- [Error model](docs/errors.md): validation failures and configuration errors.
- [Architecture](docs/architecture.md): reflection boundary and current limits.
- [Development](docs/development.md): checks, benchmarks, and runnable examples.

中文版本在每份主题文档顶部提供对应链接。

## Examples

Each example is self-contained:

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

## Scope

The core crate does not bundle Web/RPC framework integrations. Applications
choose the locale and response format, then pass validation errors to the
i18n renderer or their own adapter.
