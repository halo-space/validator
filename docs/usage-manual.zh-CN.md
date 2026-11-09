# validator 使用手册

这份手册面向第一次接入 `validator` 的 Rust 项目，覆盖安装、常用校验方式、错误处理、Schema、扩展和国际化。

## 1. 安装

项目当前通过 Git 依赖使用，不发布到 crates.io：

```toml
[dependencies]
validator = { git = "https://github.com/halo-space/validator.git" }
```

需要可复现构建时固定提交：

```toml
validator = { git = "https://github.com/halo-space/validator.git", rev = "<commit>" }
```

要求 Rust edition 2024，最低 rustc 版本为 1.97。

## 2. 结构体校验

使用 `#[derive(Validate)]` 生成校验实现：

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required, length(min = 3, max = 20))]
    name: String,

    #[validate(omitempty, email)]
    email: String,

    #[validate(gte = 0, lte = 150)]
    age: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = User {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
        age: 30,
    };

    Validator::new().validate(&user)?;
    Ok(())
}
```

一次校验可以收集多个字段错误，成功返回 `Ok(())`，失败返回 `Error::Failed`。

### 布尔值和 required

布尔字段的 `false` 也是已经提供的值：

```rust
#[derive(Debug, Validate)]
struct Settings {
    #[validate(required)]
    enabled: bool,
}
```

`enabled == false` 会通过 `required`。如果要判断字段是否为默认值，使用 `isdefault`。

`omitempty` 仍然会把 `false` 当作空值并跳过后续规则，这适合可选布尔字段。

## 3. 常用规则

规则分为几组：

- 必填：`required`、`isdefault`、`omitempty`
- 长度和数值：`length`、`min`、`max`、`range`
- 比较：`eq`、`ne`、`gt`、`gte`、`lt`、`lte`
- 字符串：`contains`、`startswith`、`endswith`、`alpha`、`numeric`、`lowercase`
- 格式：`email`、`regex`、`json`、`datetime`、`jwt`、`semver`、`hexcolor`
- 网络：`url`、`uri`、`ip`、`cidr`、`hostname`、`port`、`uuid`、`ulid`
- 集合：`unique`
- 选项：`oneof`、`oneofci`、`noneof`、`noneofci`

完整规则表见[规则参考](rules.zh-CN.md)。

## 4. 嵌套结构体和集合

嵌套校验需要显式写 `nested`：

```rust
#[derive(Debug, Validate)]
struct Profile {
    #[validate(required)]
    display_name: String,
}

#[derive(Debug, Validate)]
struct Account {
    #[validate(required, nested)]
    profile: Option<Profile>,
}
```

集合元素使用 `dive(...)`：

```rust
#[derive(Debug, Validate)]
struct Form {
    #[validate(required, min = 1, dive(required))]
    tags: Vec<String>,
}
```

错误路径会包含索引，例如 `Form.tags[1]`。Map 可以分别校验 key 和 value：

```rust
#[validate(dive(keys(max = 10), values(required)))]
labels: std::collections::HashMap<String, String>,
```

## 5. 跨字段和选择性校验

跨字段规则使用 `*_field`：

```rust
#[derive(Debug, Validate)]
struct Signup {
    password: String,

    #[validate(eq_field = "password")]
    confirm_password: String,
}
```

只校验部分字段：

```rust
let validator = Validator::new();
validator.partial(&user, ["name", "profile.email"])?;
validator.except(&user, ["password_hash"])?;
validator.filter(&user, |namespace| namespace.as_str().starts_with("profile"))?;
```

选择路径使用相对结构体字段名，不包含根类型名。

## 6. 单值校验

不需要定义结构体时，可以直接传入值和规则表达式：

```rust
let validator = Validator::new();
validator.value(&"team@example.com", "required,email")?;
validator.value(&"192.168.0.0/24", "cidr")?;
validator.value(&"550e8400-e29b-41d4-a716-446655440000", "uuid4")?;
```

规则表达式支持逗号分隔、参数和 alternative：

```text
required,length(min=3,max=20)
oneof("draft","published")
required,email|url
```

## 7. 错误处理

```rust
match Validator::new().validate(&user) {
    Ok(()) => println!("valid"),
    Err(Error::Failed(fields)) => {
        for field in fields {
            println!(
                "{} failed: {} ({})",
                field.namespace(),
                field.rule(),
                field.reason()
            );
        }
    }
    Err(error) => return Err(error.into()),
}
```

`FieldError` 提供以下信息：namespace、字段名、字段类型、规则名、底层失败原因和参数。

配置类错误包括未知规则、非法参数、Schema 错误、递归 Alias 和缺少字段上下文。

## 8. Schema 校验

规则可以放在 YAML 或 JSON 中：

```rust
use serde_json::json;
use validator::prelude::*;

let schema = Schema::from_yaml(r#"
fields:
  email:
    type: string
    rules: [required, email]
  age:
    type: uint
    rules:
      - range: {min: 0, max: 150}
"#)?;

let data = json!({
    "email": "team@example.com",
    "age": 30
});

Validator::with_schema(schema).validate_map(&data)?;
```

如果数据实现了 `serde::Serialize`，可以直接调用：

```rust
Validator::with_schema(schema).validate_serde(&user)?;
```

Schema 类型支持 `string`、`boolean`、`int`、`uint`、`float`、`array` 和 `object`。

## 9. Alias 和自定义规则

Alias 用于复用规则组合：

```rust
let validator = Validator::new()
    .alias("username", "required,length(min=3,max=20)")?;
validator.validate(&user)?;
```

自定义规则实现 `Rule`：

```rust
struct Slug;

impl Rule for Slug {
    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        Ok(field.value().string().is_some_and(|value| {
            value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        }))
    }
}

let validator = Validator::new().rule("slug", Slug)?;
```

参数化规则可以通过 `signature()` 声明参数，并在 `validate_params()` 中检查参数语义。参数预检不应依赖当前数据值，因为它可能在空值或 Schema 预编译阶段执行。

## 10. 国际化

项目内置中文和英文消息：

```rust
let messages = validator::i18n::new()
    .zh_cn()
    .en()
    .fallback("zh-CN")
    .locale("zh-CN")
    .render(fields);
```

也可以从 YAML 或 JSON 加载自定义 locale。业务层负责根据请求选择 locale 和响应格式。

## 11. 初始化和可复现构建

常规使用：

```rust
let validator = Validator::new();
```

需要显式处理内置注册表初始化错误时：

```rust
let validator = Validator::try_new()?;
let validator = Validator::try_with_schema(schema)?;
```

生产构建建议固定 Git 依赖的 `rev`、`tag` 或 `branch`，避免依赖内容无提示变化。

## 12. 进一步阅读

- [校验指南](guide.zh-CN.md)
- [规则参考](rules.zh-CN.md)
- [Schema 校验](schema.zh-CN.md)
- [扩展能力](extensions.zh-CN.md)
- [国际化](i18n.zh-CN.md)
- [错误模型](errors.zh-CN.md)
- [架构说明](architecture.zh-CN.md)
