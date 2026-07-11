# 校验指南

[English](guide.md)

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

`Valid::field` 接受直接字段或相对嵌套路径。路径根字段必须由当前结构体声明，否则校验返回 `Error::UnknownField`。
