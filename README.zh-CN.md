# validator

`validator` 是一个面向 Rust 2024 的数据校验库，基于 derive 宏、字段类型分派、动态 Schema 校验和可扩展规则注册表实现，让结构体校验、配置驱动校验、默认规则、自定义规则和错误结果保持统一。

[English](README.md)

当前以代码层声明式校验为主，同时已经支持动态 Schema 校验：

- 使用 `#[derive(Validate)]` 为结构体生成校验逻辑。
- 默认入口是 `Validator::new().validate(&value)?`。
- 单值校验入口是 `Validator::new().value(&value, "rules")?`。
- 支持 `Validator::new().alias(...)? .rule(...)?` 这种链式运行时配置。
- 内置必填、长度/范围、比较、字符串、格式、网络标识、枚举选择、颜色等常用规则。
- 支持显式嵌套结构体校验：`#[validate(nested)]`。
- 支持 Vec、数组、切片引用和 map key/value 的集合校验：`dive(...)`。
- 支持跨字段校验：`eq_field`、`ne_field`、`gt_field`、`gte_field`、`lt_field`、`lte_field`。
- 支持结构体级校验：`#[validate(check = "...")]` 和 `validator::valid::Valid`。
- 支持动态 Schema 校验：`Schema::from_yaml/json` 和 `Validator::with_schema(schema).validate_map(&data)`。
- 通过 `Error`、`FieldError`、`Namespace`、`Params` 提供稳定的错误结果。
- 支持 i18n 消息渲染：内置 `zh-CN` / `en`，并支持用户自定义 `Locale` 覆盖。

Web/RPC 框架集成不内置到核心库里；业务代码选择 locale 后，把字段错误交给 i18n 渲染即可。

## 环境要求

- Rust edition: `2024`
- 最低 rustc: `1.96.1`

## 设计说明：反射

Go 版 validator 可以直接依赖语言级运行时反射来读取结构体字段、字段类型和值。Rust 当前没有等价的内置结构体反射能力。生态里的反射库通常也要求用户额外 `derive` 一个反射 trait，库才能在运行时读取字段信息。

因此，`validator` 当前把用户 API 收敛在 `#[derive(Validate)]` 上，由 derive 宏生成校验引擎需要的轻量字段元数据和访问代码。这样用户不需要再额外写一个反射 derive，同时规则执行、`Value` 类型分派、错误结果和 i18n 仍然保持在同一套核心模型里。

这一层是内部实现细节。后续如果 Rust 本身提供成熟反射能力，或者某个反射库可以足够干净地隐藏在 `validator` 内部，我们可以把字段访问层替换成反射实现，而不改变外部的校验 DSL。

替换边界必须保持很窄：未来的 Rust 反射或 `facet` 后端只能替换 validator 如何发现字段、读取字段值，不能替换公开的 `#[validate(...)]` DSL、规则注册表、`Value` / `Kind` 语义、`Error` / `FieldError` 错误模型、Schema 规则语义和 i18n 渲染。也就是说，反射只是一种字段访问后端，不是另一套校验引擎。

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

## 单值校验

如果不需要定义结构体，可以直接校验一个值。

```rust
use validator::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let email = "alice@example.com";

    Validator::new().value(&email, "required,email")?;
    Validator::new().value(&"192.168.0.0/24", "cidr")?;
    Validator::new().value(&"1.foo.com", "hostname_rfc1123")?;
    Validator::new().value(&"550e8400-e29b-41d4-a716-446655440000", "uuid4")?;
    Validator::new().value(&"01BX5ZZKBKACTAV9WEVGEMMVRZ", "ulid")?;
    Validator::new().value(&r#"{"ok":true}"#, "json")?;
    Validator::new().value(&"2026-07-08T12:30:00+08:00", "datetime")?;
    Ok(())
}
```

单值校验失败时，错误的 namespace 和 field 都是 `$value`。

## 嵌套结构体

嵌套校验必须显式声明。只有写了 `nested`，子结构体才会执行自己的 `Validate` 实现。

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Profile {
    #[validate(required)]
    display_name: String,
}

#[derive(Debug, Validate)]
struct User {
    #[validate(nested)]
    profile: Profile,
}
```

对于 `Option<T>`，`None` 会跳过嵌套校验。如果这个可选子结构体必须存在，可以写 `required, nested`。

## 集合 Dive

当规则需要应用到集合里的每个元素时，可以使用 `dive(...)`。

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Form {
    #[validate(required, gt = 0, unique, dive(required))]
    tags: Vec<String>,
}
```

元素错误会带上集合索引，比如 `Form.tags[1]`。当前 `dive(...)` 支持 Vec、数组、切片引用和 map key/value 校验。

```rust
use std::collections::HashMap;
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Labels {
    #[validate(unique, dive(keys(max = 10), values(required)))]
    labels: HashMap<String, String>,
}
```

Map entry 错误会带上 key，比如 `Labels.labels["source"]`。
对于 Map，`unique` 校验的是 values 是否重复，因为 keys 天然唯一。

## 跨字段校验

当一个字段需要和同级字段比较时，使用 `*_field` 规则。

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Signup {
    password: String,

    #[validate(eq_field = "password")]
    confirm_password: String,
}

#[derive(Debug, Validate)]
struct Event {
    start_at: i64,

    #[validate(gt_field = "start_at")]
    end_at: i64,
}
```

当前支持 `eq_field`、`ne_field`、`gt_field`、`gte_field`、`lt_field`、`lte_field`。目标字段必须是同级 sibling field。目标字段缺失或为 `None` 时校验失败；当前字段是 `Option::None` 时会跳过非 `required` 的跨字段规则。

## 结构体级校验

当校验条件属于内置规则表达不了的业务逻辑时，可以使用结构体级 check 函数。

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
#[validate(check = "validate_draft")]
struct Draft {
    name: String,
    title: String,
}

fn validate_draft(draft: &Draft, valid: &mut validator::valid::Valid<'_>) {
    if draft.name.is_empty() && draft.title.is_empty() {
        valid
            .field("name")
            .rule("required_without")
            .param("field", "title")
            .push();
        valid
            .field("title")
            .rule("required_without")
            .param("field", "name")
            .push();
    }
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

    let error = Validator::new()
        .alias("username", "required,length(min=3,max=20)")?
        .validate(&account)
        .unwrap_err();

    assert_eq!(error.fields().unwrap().len(), 1);
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

## 动态 Schema 校验

当规则来自 YAML / JSON 配置，而不是 Rust 字段属性时，可以使用 `Schema`。

```rust
use serde_json::json;
use validator::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  email:
    type: string
    rules:
      - required
      - email
  password:
    type: string
  confirm_password:
    type: string
    rules:
      - eq_field: password
"#,
    )?;
    let data = json!({
        "email": "team@example.com",
        "password": "secret",
        "confirm_password": "secret"
    });

    Validator::with_schema(schema).validate_map(&data)?;
    Ok(())
}
```

这条路径复用同一套规则注册表、alias、`Value` 类型分派、`Error` 和 `Namespace` 模型，不是新加一套运行时引擎。

## i18n 消息渲染

校验失败后，可以把 `FieldError` 渲染成中文或英文文案。locale 来源由业务决定，validator 不读取 HTTP header 或 RPC metadata。

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Account {
    #[validate(required, email)]
    email: String,
}

fn main() {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let error = Validator::new().validate(&account).unwrap_err();

    let messages = validator::i18n::new()
        .zh_cn()
        .en()
        .fallback("zh-CN")
        .locale("zh-CN")
        .render(error.fields().unwrap());

    assert_eq!(messages[0].text, "email格式不正确");
}
```

固定中文项目可以直接使用快捷方式：`validator::i18n::zh_cn().render(fields)`。

## 内置规则

当前内置规则：

- 必填/可选: `required`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`
- Cross-field: `eq_field`, `ne_field`, `gt_field`, `gte_field`, `lt_field`, `lte_field`，用于 derive 和 Schema 校验
- Collection: `unique`
- Choice: `oneof`, `noneof`
- String: `contains`, `containsany`, `startswith`, `endswith`, `ascii`, `alpha`, `alphanum`, `numeric`, `number`, `lowercase`, `uppercase`, `boolean`
- Format: `email`, `regex`, `json`, `datetime`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`, `cmyk`
- Network: `url`, `uri`, `http_url`, `https_url`, `ip`, `ipv4`, `ipv6`, `cidr`, `cidrv4`, `cidrv6`, `hostname`, `hostname_rfc1123`, `fqdn`, `port`, `uuid`, `uuid3`, `uuid4`, `uuid5`, `ulid`
- Alias: `iscolor`

比较和尺寸类规则会根据字段类型分派：

- 字符串按字符数量比较。
- `Vec`、数组、切片、Map 按元素数量比较。
- 有符号整数、无符号整数、浮点数分别按自己的数值族处理。
- `Option::None` 会跳过非 `required` 规则，但会让 `required` 失败。

Choice 类规则会根据字段类型分派，当前支持字符串、有符号整数和无符号整数。

## 错误结果

所有公开校验入口都返回 `Error`。校验失败使用 `Error::Failed(Vec<FieldError>)`，配置错误使用 `UnknownRule`、`InvalidSchema` 等其他 `Error` 变体。

```rust
let error = Validator::new().validate(&value).unwrap_err();

for field in error.fields().unwrap_or_default() {
    println!(
        "{} failed {}",
        field.namespace().as_str(),
        field.rule()
    );
}
```

每个 `FieldError` 暴露：

- `namespace`
- `struct_namespace`
- `field`
- `struct_field`
- `kind`
- `rule`
- `reason`
- `params`

## 开发命令

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 示例

```sh
cargo run --example v1
```
