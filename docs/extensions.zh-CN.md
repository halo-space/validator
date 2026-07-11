# 扩展能力

[English](extensions.md)

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
