# Extensions

[中文](extensions.zh-CN.md)

## Alias Rules

Aliases let you name reusable rule expressions.

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

The default alias `iscolor` is available out of the box:

```rust
#[derive(Debug, Validate)]
struct Theme {
    #[validate(alias = "iscolor")]
    color: String,
}
```

Rules and aliases share one name namespace. `.rule(...)` and `.alias(...)` only
add new names; they cannot replace built-ins or existing entries. A collision
returns `Error::DuplicateName`. Aliases may reference other aliases, but direct
or indirect cycles return `Error::RecursiveAlias`; empty aliases are rejected.
Nested alias failures keep the outermost alias as `rule` and the actual failed
validation as `reason`.

Runtime alias contents are not visible to the derive macro. A derive alias must
not hide `*_field` or other field-dependent rules; use the explicit derive rule
instead. Schema aliases can contain field-dependent rules because Schema fields
are available when its execution tree is compiled.

## Custom Rules

Custom rules implement the `Rule` trait and are registered directly on
`Validator`.

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

The default `Rule::signature()` accepts no parameters. Parameterized custom
rules declare a `Signature`; derive, direct value, and Schema validation all
bind against it before `check(...)` runs. Unknown, missing, extra, or
wrong-shaped parameters return `Error::InvalidRuleExpression`.

Rules with semantic parameter constraints override
`validate_params(&Field) -> Result<(), Error>`. Every entry calls it for every
rule, alias branch, and alternative before `omitempty`, `Option::None`, an
alternative success, or an earlier rule can stop execution. It must inspect
`Params`, the declared kind, and field metadata only; it must not depend on the
current data value. `check(...)` remains the only data pass/fail operation.

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

After registration, use `#[validate(starts_with = "post-")]` directly. List
parameters use `#[validate(custom("a", "b"))]`, and named parameters use
`#[validate(custom(min = 1, max = 10))]`; the Rule `Signature` decides which
shape is valid.

`Signature` supports no parameters, text, optional text, lists, fixed named
parameters, and field-condition pairs. Bound `Params` preserve those shapes
and expose them through `text(...)`, `list(...)`, and `pairs(...)`; rules do not
split comma-encoded strings.

Custom single-target cross-field rules use the `*_field` suffix and an explicit
field Signature:

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

Register it with `.rule("same_field", SameField)?`. Direct value validation
returns `MissingFieldContext`; derive and Schema provide the declared sibling.
For custom logic involving several fields, use a struct-level `check`.
