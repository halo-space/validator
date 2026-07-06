# validator

`validator` 是一个面向 Rust 2024 的数据校验库，当前重点是代码层声明式校验。它基于 derive 宏、字段类型分派和可扩展规则注册表实现，让结构体校验、默认规则、自定义规则和错误结果保持统一。

[English](README.md)

当前已实现的能力：

- 使用 `#[derive(Validate)]` 为结构体生成校验逻辑。
- 默认入口是 `Validator::new().validate(&value)?`。
- 支持 `Validator::new().alias(...)? .rule(...)?` 这种链式运行时配置。
- 内置必填、长度/范围、比较、字符串、格式、颜色、URL、枚举选择等常用规则。
- 通过 `Errors`、`FieldError`、`Namespace`、`Args` 提供稳定的错误结果。

运行时 Schema 校验、嵌套结构体递归、集合 `dive(...)` 和 i18n 会单独设计推进，不属于当前第一版代码能力。

## 环境要求

- Rust edition: `2024`
- 最低 rustc: `1.96`

## 基本用法

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required, length(min = 3, max = 20))]
    name: String,

    #[validate(omitempty, email)]
    email: String,

    #[validate(gte = 0, lte = 130)]
    age: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = User {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
        age: 42,
    };

    Validator::new().validate(&user)?;
    Ok(())
}
```

## Alias 规则

Alias 用来给一组规则起名字，适合复用常见校验组合。

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Account {
    #[validate(alias = "username")]
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let account = Account {
        name: "al".to_owned(),
    };

    let errors = Validator::new()
        .alias("username", "required,length(min=3,max=20)")?
        .validate(&account)
        .unwrap_err();

    assert_eq!(errors.len(), 1);
    Ok(())
}
```

默认内置 alias `iscolor` 可直接使用：

```rust
#[derive(Debug, Validate)]
struct Theme {
    #[validate(alias = "iscolor")]
    color: String,
}
```

## 自定义规则

自定义规则实现 `Rule` trait，然后注册到 `Validator` 上。

```rust
use validator::prelude::*;

struct Slug;

impl Rule for Slug {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .map(|value| {
                value
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Validate)]
struct Post {
    #[validate(alias = "slug_alias")]
    slug: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let post = Post {
        slug: "hello-rust".to_owned(),
    };

    Validator::new()
        .alias("slug_alias", "slug")?
        .rule("slug", Slug)?
        .validate(&post)?;

    Ok(())
}
```

## 内置规则

当前内置规则：

- Presence: `required`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `gt`, `gte`, `lt`, `lte`
- Choice: `oneof`
- String: `contains`, `startswith`, `endswith`, `alpha`, `alphanum`, `numeric`, `number`, `lowercase`, `uppercase`, `boolean`
- Format: `email`, `regex`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`, `cmyk`
- Network: `url`
- Alias: `iscolor`

比较和尺寸类规则会根据字段类型分派：

- 字符串按字符数量比较。
- `Vec`、数组、切片、Map 按元素数量比较。
- 有符号整数、无符号整数、浮点数分别按自己的数值族处理。
- `Option::None` 会跳过非 `required` 规则，但会让 `required` 失败。

## 错误结果

校验失败会返回 `Errors`，其中包含每个字段规则失败对应的 `FieldError`。

```rust
let errors = Validator::new().validate(&value).unwrap_err();

for error in errors.iter() {
    println!(
        "{} failed {}",
        error.namespace().as_str(),
        error.rule()
    );
}
```

每个 `FieldError` 暴露：

- `namespace`
- `struct_namespace`
- `field`
- `struct_field`
- `rule`
- `actual_rule`
- `args`

## 开发命令

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
