# validator

`validator` 是一个面向 Rust 2024 的数据校验库，基于 derive 宏、字段类型分派、动态 Schema 校验和可扩展规则注册表实现，让结构体校验、配置驱动校验、默认规则、自定义规则和错误结果保持统一。

[English](README.md)

当前以代码层声明式校验为主，同时已经支持动态 Schema 校验：

- 使用 `#[derive(Validate)]` 为结构体生成校验逻辑。
- 默认入口是 `Validator::new().validate(&value)?`。
- 支持通过 `partial(...)`、`except(...)` 和正向 `filter(...)` 选择 derive 字段。
- 单值校验入口是 `Validator::new().value(&value, "rules")?`。
- 支持 `Validator::new().alias(...)? .rule(...)?` 这种链式运行时配置。
- 内置必填、长度/范围、比较、字符串、格式、网络标识、枚举选择、颜色等常用规则。
- 支持显式嵌套结构体校验：`#[validate(nested)]`。
- 支持 Vec、数组、切片引用和 map key/value 的集合校验：`dive(...)`。
- 支持跨字段校验：`eq_field`、`ne_field`、`gt_field`、`gte_field`、`lt_field`、`lte_field`。
- 支持条件字段校验：`required_if`、`required_unless`、`skip_unless`、`required_with`、`required_with_all`、`required_without`、`required_without_all` 和 `excluded_*` 规则。
- 支持结构体级校验：`#[validate(check = "...")]` 和 `validator::valid::Valid`。
- 支持动态 Schema 校验：`Schema::from_yaml/json`、`Validator::with_schema(schema).validate_map(&data)`，以及用于 `serde::Serialize` 数据的 `validate_serde(&value)`。
- 通过 `Error`、`FieldError`、`Namespace`、`Params` 提供稳定的错误结果。
- 支持 i18n 消息渲染：内置 `zh-CN` / `en`，并支持用户自定义 `Locale` 覆盖。

Web/RPC 框架集成不内置到核心库里；业务代码选择 locale 后，把字段错误交给 i18n 渲染即可。

## 环境要求

- Rust edition: `2024`
- 最低 rustc: `1.97`

## Git 依赖使用

当前项目先按 Git 依赖方式使用，不发布到 crates.io。

```toml
[dependencies]
validator = { git = "ssh://git@github-halo/halo-space/validator.git" }
```

实际项目里可以换成自己环境可访问的 Git URL。需要可复现构建时，建议固定 `rev`、`tag` 或 `branch`。

## 设计说明：反射

项目目前处于初始开发阶段，不保留旧 API 或旧配置写法。类型名、规则名或 API 边界设计错误时，直接替换实现，不增加兼容 alias、deprecated wrapper 或回退解析。出现需要兼容补丁的提议时，优先重新检查底层架构，而不是把兼容分支叠加到现有设计上。

Go 版 validator 可以直接依赖语言级运行时反射来读取结构体字段、字段类型和值。Rust 当前没有等价的内置结构体反射能力。生态里的反射库通常也要求用户额外 `derive` 一个反射 trait，库才能在运行时读取字段信息。

因此，`validator` 当前把用户 API 收敛在 `#[derive(Validate)]` 上，由 derive 宏生成校验引擎需要的轻量字段元数据和访问代码。这样用户不需要再额外写一个反射 derive，同时规则执行、`Value` 类型分派、错误结果和 i18n 仍然保持在同一套核心模型里。
生成的访问代码是按需的：只有校验属性实际引用的直接字段和完整嵌套目标才会进入访问层。

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

## 选择性校验

derive 模型可以只校验部分字段，不需要复制一套结构体或规则：

```rust
let validator = Validator::new();

validator.partial(&user, ["name", "profile.email"])?;
validator.except(&user, ["password_hash"])?;
validator.filter(&user, |namespace| {
    matches!(namespace.as_str(), "profile" | "profile.email")
})?;
```

selector 使用相对的 Rust struct namespace，不包含根类型名。嵌套字段使用点分路径，
集合元素使用 `items[0].email` 这样的具体 namespace，Map entry 使用错误结果中相同的
带引号 key 形式。选中父字段表示校验完整子树；排除父字段表示跳过完整子树，排除单个
子字段不会影响它的兄弟字段。

`filter` 使用正向语义：返回 `true` 表示校验这个字段或 entry，返回 `false` 表示跳过
它和它的子节点。需要保留嵌套字段时，回调也必须对它的祖先返回 `true`。同一个
namespace 可能被回调多次，业务逻辑不能依赖调用次数。struct-level error 使用同一套
选择范围过滤。空 `partial` 表示不校验任何字段，空 `except` 等价于完整校验；不支持
通配符路径。每个非空选择路径都必须匹配已声明字段或集合 entry，否则返回
`Error::UnknownField`。选择性校验由 `#[derive(Validate)]` 生成；手写 `Validate` 实现只
支持完整校验。

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
运行时规则表达式采用严格语法：空规则、空 alternative、括号不平衡、引号未闭合或悬空转义都会返回 `Error::InvalidRuleExpression`。引号内的 `\\` 用来转义下一个字符；参数本身需要一个反斜杠时应写成 `\\\\`。

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

元素错误会带上集合索引，比如 `Form.tags[1]`。当前 `dive(...)` 支持 Vec、数组、切片引用和 map key/value 校验。元素规则会在遍历前根据声明的 `Value` 类型完成参数预检，因此空集合也不能隐藏错误的规则配置。

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
浮点去重遵循 equality 语义：多个 NaN 互不重复，`0.0` 与 `-0.0` 重复。已知的标量 Kind 会在参数预检阶段被拒绝，因此 `omitempty` 不能隐藏错误的 `unique` 配置。

结构体集合可以按元素的一个或多个相对字段去重：

```rust
use validator::prelude::*;

#[derive(Debug)]
struct Profile {
    email: String,
}

#[derive(Debug)]
struct Member {
    tenant_id: u64,
    profile: Profile,
}

#[derive(Debug, Validate)]
struct Team {
    #[validate(unique = ["tenant_id", "profile.email"])]
    members: Vec<Member>,
}
```

`unique = "email"` 仍然是 `unique = ["email"]` 的单字段简写。错误参数统一为 `fields = ["email"]` 或声明的复合字段列表。路径相对于集合元素解析，只有末端字段需要实现 `Value`。重复错误位于 `Team.members`。Map 投影、`dive(...)` 内投影和原生 `Validator::value(&members, "unique=email")` 没有元素字段上下文；单值入口会返回 `MissingFieldContext`。

原生集合通过 `Value` 参与 `dive(...)` 外层的通用规则。字符串、数字等内置标量已经实现了 `Value`；自定义元素类型的集合如果要使用无参数 `unique` 等需要读取元素本身的规则，需要先为元素实现 `Value`：

```rust
use validator::prelude::*;

#[derive(Debug)]
struct Item {
    id: u64,
}

impl Value for Item {
    fn kind(&self) -> Kind {
        Kind::Uint(UintKind::U64)
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Uint(UintKind::U64))
    }

    fn required(&self) -> bool {
        self.id != 0
    }

    fn uint(&self) -> Option<u128> {
        Some(u128::from(self.id))
    }
}

#[derive(Debug, Validate)]
struct Basket {
    #[validate(required, min = 1, unique)]
    items: Vec<Item>,
}
```

这是当前静态 `Value` 分派模型的明确边界，不使用运行时反射或隐藏回退。字段如果只使用 `dive(nested)`，则不受这个约束影响，集合元素只需要实现 `Validate`。

对于 JSON 这类类型确实由运行时数据决定的值，`Value::declared_kind()` 默认返回 `None`。静态自定义类型如果会放在 `Option` 或 `dive(...)` 中，应返回自己的声明类型；这样即使当前值是 `None` 或集合为空，参数预检仍然能按正确类型执行。

## 跨字段校验

当一个字段需要和同级字段或向下嵌套字段比较时，使用 `*_field` 规则。

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

struct Contact {
    email: String,
}

struct Profile {
    contact: Contact,
}

#[derive(Debug, Validate)]
struct Account {
    profile: Option<Profile>,

    #[validate(eq_field = "profile.contact.email")]
    email: String,
}
```

当前支持 `eq_field`、`ne_field`、`gt_field`、`gte_field`、`lt_field`、`lte_field`。单段目标仍然表示同级 sibling；点分目标相对当前属性所在结构体向下解析。路径中的每个 `Option<T>` 都会自动借用；任一目标段为 `None` 时都视为目标缺失，包括 `ne_field` 在内的比较规则都会失败。当前字段是 `Option::None` 时仍会跳过非 `required` 的跨字段规则。

只有终点字段需要实现 `Value`，中间结构体不需要实现 `Value` 或 `Validate`。读取路径不会触发嵌套校验；只有显式 `#[validate(nested)]` 才会校验嵌套结构体自身。路径只接受规范的点分 Rust 字段名，不支持父级/root、数组索引、Map key 或通配符。raw 字段 `r#type` 写成 `"type"`。

Equality 与 ordering 分开处理：字符串 equality 比较内容，字符串 ordering 按 Unicode 字符数量比较；集合按元素数量比较，数字和 `SystemTime` 按各自类型族比较，bool 只支持 `eq_field` / `ne_field`。浮点 NaN 不相等也不可排序。

条件字段规则仍然只使用同级字段：

```rust
#[derive(Debug, Validate)]
struct Post {
    status: String,
    email: String,
    phone: String,

    #[validate(required_if(status = "published"))]
    published_at: Option<String>,

    #[validate(required_unless(status = "draft"))]
    title: String,

    #[validate(skip_unless(status = "published"))]
    reviewer: String,

    #[validate(required_with("email", "phone"))]
    contact_name: String,

    #[validate(required_with_all("email", "phone"))]
    all_contact_name: String,

    #[validate(required_without("email", "phone"))]
    fallback_contact: String,

    #[validate(required_without_all("email", "phone"))]
    all_fallback_contact: String,

    #[validate(excluded_if(status = "archived"))]
    archive_note: String,
}
```

当前支持 `required_if`、`required_unless`、`skip_unless`、`required_with`、`required_with_all`、`required_without`、`required_without_all`、`excluded_if`、`excluded_unless`、`excluded_with`、`excluded_with_all`、`excluded_without`、`excluded_without_all`，用于 derive 和 Schema 校验。`*_if` / `*_unless` 会按字段类型比较同级字段值；`*_with` / `*_without` 会判断同级字段是否存在且非空。带 `_all` 的变体要求所有引用字段都满足条件；不带 `_all` 的字段列表变体只要任一引用字段满足条件就会触发。`excluded_*` 规则在条件触发时要求当前字段为空。`skip_unless` 按 Go 当前行为实现：所有键值条件都匹配时，当前字段必须有值；否则该规则通过。

当 `*_if` 或 `*_unless` 需要匹配缺失字段或 `Option::None` 时，使用带引号的字符串 `"null"`；有明确数值类型的字段也使用同一哨兵。

## SystemTime 校验

原生时间校验只支持 Rust 标准库的 `std::time::SystemTime`，也就是 Rust 里最接近 Go `time.Time` 使用场景的标准时间点类型。

```rust
use std::time::{Duration, SystemTime};
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Event {
    #[validate(lte)]
    created_at: SystemTime,

    #[validate(gt)]
    expires_at: SystemTime,

    #[validate(gt_field = "created_at")]
    updated_at: SystemTime,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now();
    let event = Event {
        created_at: now.checked_sub(Duration::from_secs(60)).unwrap(),
        expires_at: now + Duration::from_secs(60),
        updated_at: now + Duration::from_secs(1),
    };

    Validator::new().validate(&event)?;
    Validator::new().value(&event.created_at, "lte")?;
    Ok(())
}
```

对于 `SystemTime`，无参数的 `lt`、`lte`、`gt`、`gte` 会和本次校验开始时捕获的同一个 `now` 比较。`#[validate(gt = "2026-07-08T00:00:00Z")]` 这种字面量参数会作为配置错误返回，因为 `SystemTime` 本身没有统一内置字符串格式。`eq` / `ne` 不和当前时间比较；时间相等性请使用 `eq_field` / `ne_field`。

`datetime` 仍然只是字符串格式校验。动态 Schema 不支持原生 `type: time`；如果要校验时间字符串，用 `type: string` 加 `datetime`。

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

Rule 和 Alias 共用同一个名称空间。`.rule(...)` 与 `.alias(...)` 只能注册新名称，不能覆盖内置项或已有项；名称冲突返回 `Error::DuplicateName`。Alias 可以引用其他 Alias，但直接或间接循环引用会返回 `Error::RecursiveAlias`，空 Alias 会被拒绝。多层 Alias 失败时，`rule` 保留最外层 Alias，`reason` 保留实际失败规则。

运行时 Alias 的内容对 derive 宏不可见，因此 derive Alias 不能隐藏 `*_field` 等需要字段访问的规则，应直接在字段属性上写出该规则。Schema 编译执行树时已经拥有字段定义，所以 Schema Alias 可以包含跨字段规则。

## 自定义规则

自定义规则实现 `Rule` trait，然后注册到 `Validator` 上。

```rust
use validator::prelude::*;

struct Slug;

impl Rule for Slug {
    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        Ok(field
            .value()
            .string()
            .map(|value| {
                value
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
            })
            .unwrap_or(false))
    }
}

#[derive(Debug, Validate)]
struct Post {
    #[validate(slug)]
    slug: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let post = Post {
        slug: "hello-rust".to_owned(),
    };

    Validator::new()
        .rule("slug", Slug)?
        .validate(&post)?;

    Ok(())
}
```

默认 `Rule::signature()` 是无参数规则。带参数的自定义规则必须显式声明 `Signature`，所有 derive、单值和 Schema 入口都会先按该契约严格绑定参数；未知、缺少、多余或类型不匹配的参数会作为 `Error::InvalidRuleExpression` 返回，不会进入 `check(...)`。

如果参数还有取值语义约束，Rule 应实现 `validate_params(&Field) -> Result<(), Error>`。所有入口都会在 `omitempty`、`Option::None`、alternative 成功或前序规则停止执行之前，递归预检每条 Rule、Alias 和 alternative。这个方法只检查 `Params`、声明类型和字段元数据，不能依赖当前数据值；数据是否通过仍然只由 `check(...)` 判断。

```rust
struct StartsWith;

impl Rule for StartsWith {
    fn signature(&self) -> Signature {
        Signature::text("prefix")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), Error> {
        let prefix = field.params().text("prefix").expect("Signature binds prefix");
        if prefix.is_empty() {
            return Err(Error::InvalidRuleExpression {
                expression: "starts_with".to_owned(),
                reason: "prefix must not be empty".to_owned(),
            });
        }
        Ok(())
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let Some(prefix) = field.params().text("prefix") else {
            return Ok(false);
        };
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.starts_with(prefix)))
    }
}
```

注册后可以直接写 `#[validate(starts_with = "post-")]`。列表参数使用 `#[validate(custom("a", "b"))]`，命名参数使用 `#[validate(custom(min = 1, max = 10))]`；最终允许哪种参数形态由该 Rule 的 `Signature` 决定。

`Signature` 支持无参数、单个文本、可选文本、列表、固定命名参数和字段条件对。绑定后的 `Params` 保留结构边界，通过 `text(...)`、`list(...)` 和 `pairs(...)` 读取，不需要按逗号拆分字符串。

自定义单目标跨字段规则统一使用 `*_field` 后缀和显式字段 Signature：

```rust
struct SameField;

impl Rule for SameField {
    fn signature(&self) -> Signature {
        Signature::field("compare")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let target = field.params().text("compare").unwrap();
        Ok(field.sibling(target).and_then(Value::string) == field.value().string())
    }
}

#[derive(Validate)]
struct Pair {
    left: String,
    #[validate(same_field = "left")]
    right: String,
}
```

通过 `.rule("same_field", SameField)?` 注册。单值入口没有同级字段，会返回 `MissingFieldContext`；derive 和 Schema 会提供显式声明的目标字段。涉及多个字段的复杂自定义逻辑继续使用 struct-level `check`。

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

这条路径复用同一套规则注册表、alias、`Value` 类型分派、`Error` 和 `Namespace` 模型，不是另一套校验引擎。

Schema 的单目标字段规则使用同一套相对点分语法：

```yaml
fields:
  profile:
    type: object
    fields:
      email:
        type: string
  email:
    type: string
    rules:
      - eq_field: profile.email
```

每个中间 Schema 段都必须声明为 `object`；这项能力不会穿过 array。运行时中间段缺失或为 null 时，目标按缺失处理，同时保留终点字段声明的 `Kind`。Schema alias 可以包含点分字段规则，因为 alias 展开和路径校验都发生在 Schema Tree 编译阶段。单段目标按序列化字段名精确匹配，因此 `source-url` 这类名称仍然有效；包含 `.` 的目标固定表示嵌套路径。如果同一层还声明了同名字面点分字段，Schema 编译会直接拒绝歧义配置，而不是按查找优先级选择其中一个。

Schema 配置采用严格模式：顶层只允许 `fields`，字段定义只允许 `type`、`rules` 和 `fields`。`type` 只接受 `string`、`boolean`、`int`、`uint`、`float`、`array`、`object`，只有 `object` 和 `array` 可以定义嵌套 `fields`。即使 `fields: {}` 为空，它仍然表示结构声明：省略 `type` 时推断为 `object`，标量类型配置空 `fields` 仍会报错，array 配置空 `fields` 时每个元素仍必须是 object；完全没有 `fields` 的 array 不限制元素必须是 object。未知键或未知类型返回 `Error::InvalidSchema`；不符合 Rule `Signature` 的参数在 Schema 编译时返回 `Error::InvalidRuleExpression`。语义参数也会在缓存 Schema Tree 构建时完成预检，早于根数据或字段数据的类型检查短路。

Schema 面向 JSON/YAML 数据。它支持把 `datetime` 当成字符串规则使用，但不支持原生 `SystemTime` 值，也不支持 `type: time`。数字规则按 Schema 声明族执行：`int` 使用有符号整数，`uint` 使用无符号整数，`float` 只接受浮点形式。整数值不能通过 `type: float`，浮点值也不能通过 `type: int`。

Schema 类型名与校验规则名属于不同命名空间。`type: float` 声明字段的数据类型；`number` 是校验谓词，不是类型名。`number` 接受原生数值，以及只包含 ASCII 数字的字符串，例如 `"12345"`，但不接受正负号和小数点。`numeric` 同样接受原生数值，它的字符串形式还可以包含前导正负号和小数部分，例如 `"-12.5"`。因此 `number` 可以写在 `rules` 中，但不能写成 Schema `type`。

`type: array` 下的 `fields` 描述对象元素，可以直接按元素字段去重：

```yaml
fields:
  users:
    type: array
    rules:
      - unique: [tenant_id, profile.email]
    fields:
      tenant_id:
        type: uint
      profile:
        type: object
        fields:
          email:
            type: string
            rules: email
```

元素字段错误使用 `users[0].email` namespace；包括 `null` 在内的非对象元素使用 `users[0]` namespace 返回 `type` 错误。`unique: email` 是单字段列表简写。如果单字段或复合字段投影到的输入不符合子字段类型，unique 会跳过该元素的完整键，由子字段返回带索引的 `type` 错误；错误输入不会被误判成规则配置错误。

如果数据已经实现了 `serde::Serialize`，可以直接使用 `validate_serde(...)`。
Schema 字段名跟随序列化后的数据结构，包括 `serde(rename)`、`serde(rename_all)`、`serde(skip_serializing_if)` 和 `serde(flatten)`。
`validate_serde` 会先取得并编译 Schema，再调用用户的序列化实现，因此 `MissingSchema` 或非法 Schema 配置不会被自定义 serializer 错误遮住。

```rust
use serde::Serialize;
use validator::prelude::*;

#[derive(Serialize)]
struct User {
    #[serde(rename = "user_name")]
    name: String,
    email: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  user_name:
    type: string
    rules:
      - required
  email:
    type: string
    rules:
      - email
"#,
    )?;
    let user = User {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
    };

    Validator::with_schema(schema).validate_serde(&user)?;
    Ok(())
}
```

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

用户自定义 locale 可以从 YAML 或 JSON 加载：

```rust
let zh = validator::i18n::Locale::from_yaml(
    r#"
locale: zh-CN
rules:
  email: "请输入正确的{field}"
fields:
  email: "邮箱"
"#,
)?;

let messages = validator::i18n::new()
    .zh_cn()
    .use_locale(zh)
    .fallback("zh-CN")
    .locale("zh-CN")
    .render(error.fields().unwrap());
```

Locale 资源同样采用严格模式，未知键会返回 `Error::InvalidData`。
语言标识只使用 `locale` 键；`name` 不作为兼容写法。代码构造的语言标识可以通过 `Locale::locale()` 读取。

## 内置规则

当前内置规则：

- 必填/可选: `required`, `isdefault`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `eq`, `ne`, `eq_ignore_case`, `ne_ignore_case`, `gt`, `gte`, `lt`, `lte`
- Field-aware: `eq_field`, `ne_field`, `gt_field`, `gte_field`, `lt_field`, `lte_field`, `fieldcontains`, `fieldexcludes`, `required_if`, `required_unless`, `skip_unless`, `required_with`, `required_with_all`, `required_without`, `required_without_all`, `excluded_if`, `excluded_unless`, `excluded_with`, `excluded_with_all`, `excluded_without`, `excluded_without_all`，用于 derive 和 Schema 校验
- Collection: `unique`
- Choice: `oneof`, `oneofci`, `noneof`, `noneofci`
- String: `contains`, `containsany`, `containsrune`, `excludes`, `excludesall`, `excludesrune`, `startswith`, `endswith`, `startsnotwith`, `endsnotwith`, `ascii`, `printascii`, `multibyte`, `alpha`, `alphaspace`, `alphaunicode`, `alphanum`, `alphanumspace`, `alphanumunicode`, `numeric`, `number`, `lowercase`, `uppercase`, `boolean`

  这里的 `number` 表示字符串的 ASCII 数字谓词，不是 Schema 类型；`numeric` 还允许正负号和小数部分。
- Format: `email`, `regex`, `json`, `datetime`, `e164`, `base32`, `base64`, `base64url`, `base64rawurl`, `hexadecimal`, `url_encoded`, `html`, `html_encoded`, `jwt`, `mac`, `semver`, `origin`, `datauri`, `latitude`, `longitude`, `ssn`, `md4`, `md5`, `sha256`, `sha384`, `sha512`, `ripemd128`, `ripemd160`, `tiger128`, `tiger160`, `tiger192`, `eth_addr`, `mongodb`, `mongodb_connection_string`, `dns_rfc1035_label`, `cve`, `cron`, `ein`, `bic_iso_9362_2014`, `bic`, `isbn`, `isbn10`, `isbn13`, `issn`, `credit_card`, `luhn_checksum`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`, `cmyk`
- Network: `url`, `uri`, `http`, `https`, `ip`, `ipv4`, `ipv6`, `cidr`, `cidrv4`, `cidrv6`, `hostname`, `hostname_port`, `hostname_rfc1123`, `fqdn`, `port`, `uuid`, `uuid3`, `uuid4`, `uuid5`, `uuid_rfc4122`, `uuid3_rfc4122`, `uuid4_rfc4122`, `uuid5_rfc4122`, `ulid`, `tcp4`, `tcp6`, `tcp`, `udp4`, `udp6`, `udp`
- Alias: `iscolor`

有序比较和尺寸类规则会根据字段类型分派：

- 字符串按字符数量比较。
- `Vec`、数组、切片、Map 按元素数量比较。
- 有符号整数、无符号整数、浮点数分别按自己的数值族处理。
- `std::time::SystemTime` 支持无参数时间比较，也支持同类型 `*_field` 字段间比较。
- `Option::None` 会跳过非 `required` 规则，但会让 `required` 失败。

Equality 规则比较字符串内容，不比较长度。`length` 不允许空参数，也不允许 `exact` 与 `min` / `max` 混用；`length` 和 `range` 会在参数预检阶段拒绝倒置边界。

Choice 类规则会根据字段类型分派，当前支持字符串、有符号整数和无符号整数。
URL 和 URI 使用结构化解析器；`hostname` 遵循 RFC952，`hostname_rfc1123` 允许数字开头，`fqdn` 要求非数字顶级域名。
`cidr` 接受 IPv4 或 IPv6 address-prefix，`cidrv4` 额外要求 canonical network address。`mac` 支持 6、8、20 octet 链路层地址，`uuid4` / `uuid5` 的小写规则同时检查 version 和 RFC variant。

## 当前边界

这些是当前 API 有意保留的边界：

- 参数化 `unique` 支持 Vec、数组、切片引用和 Schema object arrays 的单字段、复合字段及嵌套元素路径；不支持 Map、原生 direct value，也不能写在 `dive(...)` 内。
- 嵌套字段目标只支持相对当前结构体向下解析；不支持父级/root、集合索引、Map key，也不支持条件规则路径。
- derive 代码必须显式写出元素字段路径，运行时 alias 无法生成 Rust 字段访问；Schema 在运行时已有字段定义，因此 Schema alias 可以包含参数化 unique 表达式。
- 原生集合使用 `dive(...)` 之外的通用规则时，元素或 Map value 类型必须实现 `Value`；纯 `dive(nested)` 只要求元素实现 `Validate`。
- 原生时间校验只支持 `std::time::SystemTime`；不内置 `Duration`、`chrono`、`time` crate 值。
- 暂不内置 `country_code` 以及相关国家代码 alias。
- 不内置 Web/RPC 框架集成；业务或外部 adapter 决定 locale 和响应格式。
- 不要求用户使用 Rust 运行时反射。字段访问由 `#[derive(Validate)]` 生成，并保持在公开校验 DSL 背后。

## 错误结果

所有公开校验入口都返回 `Error`。校验失败使用 `Error::Failed(Vec<FieldError>)`；未知规则、重复名称、错误参数、缺少字段上下文、递归 Alias、Schema 或数据转换错误使用对应的配置错误变体。参数配置预检始终先于数据相关短路，因此字段缺失、为空或被跳过时，错误参数仍会直接返回。

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
cargo bench --bench validation
cargo package --manifest-path derive/Cargo.toml --allow-dirty
```

benchmark 覆盖 derive 成功/失败、选择性校验、direct expression 冷/热路径、
Schema 冷/热路径、`validate_serde`、集合 dive 和 compound unique 投影。
本地快速检查可以运行 `cargo bench --bench validation -- --quick`。

当前分发路径是 Git 依赖使用，暂不处理 registry 发布。根 `validator` crate 这一阶段不按 crates.io package 方式收口；这里只对技术包 `validator-derive` 做 `cargo package` 检查。

## 示例

```sh
cargo run --example v1
```
