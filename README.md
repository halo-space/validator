# validator

`validator` is a Rust 2024 validation library built around Rust-friendly derive
macros, typed value dispatch, dynamic Schema validation, and an extensible rule
registry.

[中文文档](README.zh-CN.md)

The current implementation is centered on code-level validation and also supports dynamic Schema validation:

- `#[derive(Validate)]` for struct validation.
- `Validator::new().validate(&value)?` as the default entry point.
- `Validator::new().value(&value, "rules")?` for direct single-value validation.
- Chainable runtime configuration with `Validator::new().alias(...)? .rule(...)?`.
- Built-in rules for required values, size checks, comparisons, strings,
  formats, network identifiers, choices, and colors.
- Explicit nested struct validation with `#[validate(nested)]`.
- Collection validation with `dive(...)` for Vec, arrays, slices, and map
  key/value pairs.
- Cross-field validation with `eq_field`, `ne_field`, `gt_field`, `gte_field`,
  `lt_field`, and `lte_field`.
- Struct-level validation with `#[validate(check = "...")]` and
  `validator::valid::Valid`.
- Dynamic Schema validation with `Schema::from_yaml/json`,
  `Validator::with_schema(schema).validate_map(&data)`, and
  `validate_serde(&value)` for `serde::Serialize` values.
- Consistent error reporting through `Error`, `FieldError`, `Namespace`, and
  `Params`.
- i18n message rendering with built-in `zh-CN` / `en` locales and custom
  `Locale` overrides.

Built-in Web/RPC framework integrations are intentionally out of scope. The
application chooses the locale and passes field errors to i18n rendering.

## Requirements

- Rust edition: `2024`
- Minimum supported rustc: `1.96.1`

## Design Note: Reflection

Go validators can lean on language-level runtime reflection to inspect struct
fields, field types, and field values. Rust does not currently provide an
equivalent built-in reflection model for ordinary structs. Existing ecosystem
options require users to derive an additional reflection trait before a library
can inspect fields at runtime.

For that reason, `validator` keeps the user-facing API centered on
`#[derive(Validate)]` and lets the derive macro generate the small amount of
field metadata and access code needed by the validation engine. This avoids
requiring users to derive a separate reflection trait while keeping rule
execution, `Value` dispatch, errors, and i18n centralized.

This layer is intentionally an internal implementation detail. If Rust gains a
stable reflection story, or a reflection crate becomes mature enough to hide
cleanly behind `validator`, the field-access layer can be reworked to use that
reflection backend without changing the validation DSL.

The replacement boundary is deliberately narrow: a future reflection or `facet`
backend may replace how validator discovers fields and reads field values, but
it must not replace the public `#[validate(...)]` DSL, rule registry, `Value` /
`Kind` semantics, `Error` / `FieldError` model, Schema rule semantics, or i18n
rendering. In other words, reflection is only a field-access backend, not a new
validation engine.

## Usage

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

## Direct Value Validation

Use `value(...)` when a struct is unnecessary.

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

Direct value failures use `$value` as their namespace and field name.

## Nested Structs

Nested validation is explicit. Use `nested` when a child struct should run its
own `Validate` implementation.

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

For `Option<T>`, `None` skips nested validation. Use `required, nested` when
the optional child must be present.

## Collection Dive

Use `dive(...)` when rules should apply to each collection element.

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Form {
    #[validate(required, gt = 0, unique, dive(required))]
    tags: Vec<String>,
}
```

Element errors include the collection index, such as `Form.tags[1]`. `dive(...)`
supports Vec, arrays, slice references, and map key/value validation.

```rust
use std::collections::HashMap;
use validator::prelude::*;

#[derive(Debug, Validate)]
struct Labels {
    #[validate(unique, dive(keys(max = 10), values(required)))]
    labels: HashMap<String, String>,
}
```

Map entry errors include the key, such as `Labels.labels["source"]`.
For maps, `unique` checks map values because map keys are already unique.

## Cross-Field Validation

Use `*_field` rules when one field should be compared with a sibling field.

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

Supported rules are `eq_field`, `ne_field`, `gt_field`, `gte_field`,
`lt_field`, and `lte_field`. Target names are same-level sibling fields. Missing
or `None` target values fail validation; a current `Option::None` skips the
cross-field rule unless `required` is also present.

## Struct-Level Validation

Use a struct-level check when validation depends on custom business logic that
does not fit a built-in rule.

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

## Custom Rules

Custom rules implement the `Rule` trait and are registered directly on
`Validator`.

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

## Dynamic Schema Validation

Use `Schema` when rules come from YAML or JSON instead of Rust field
attributes.

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

This path reuses the same rule registry, aliases, `Value` dispatch, `Error`,
and `Namespace` model as code-level validation.

If the data already implements `serde::Serialize`, use `validate_serde(...)`.
Schema field names follow the serialized data shape, including
`serde(rename)`, `serde(rename_all)`, `serde(skip_serializing_if)`, and
`serde(flatten)`.

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

## i18n Message Rendering

Validation failures can be rendered into localized messages. The application
chooses the locale; validator does not read HTTP headers or RPC metadata.

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
        .locale("en")
        .render(error.fields().unwrap());

    assert_eq!(messages[0].text, "email must be a valid email address");
}
```

For fixed Chinese output, use the shortcut: `validator::i18n::zh_cn().render(fields)`.

Custom locale resources can be loaded from YAML or JSON:

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

## Built-In Rules

Current built-in rules:

- Required/Optional: `required`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`
- Cross-field: `eq_field`, `ne_field`, `gt_field`, `gte_field`, `lt_field`,
  `lte_field` for derive and Schema validation
- Collection: `unique`
- Choice: `oneof`, `noneof`
- String: `contains`, `containsany`, `startswith`, `endswith`, `ascii`,
  `alpha`, `alphanum`, `numeric`, `number`, `lowercase`, `uppercase`,
  `boolean`
- Format: `email`, `regex`, `json`, `datetime`, `hexcolor`, `rgb`, `rgba`,
  `hsl`, `hsla`, `cmyk`
- Network: `url`, `uri`, `http_url`, `https_url`, `ip`, `ipv4`, `ipv6`,
  `cidr`, `cidrv4`, `cidrv6`, `hostname`, `hostname_rfc1123`, `fqdn`,
  `port`, `uuid`, `uuid3`, `uuid4`, `uuid5`, `ulid`
- Alias: `iscolor`

Comparison and size rules dispatch by field type:

- Strings use character count.
- Vectors, arrays, slices, and maps use item count.
- Signed integers, unsigned integers, and floats use their own numeric families.
- `Option::None` skips non-`required` rules and fails `required`.

Choice rules dispatch by field type for strings, signed integers, and unsigned integers.

## Current Limits

These are intentional limits in the current API surface:

- `unique` supports whole collections and map values, but not `unique=field`.
- `country_code` and related country aliases are not built in yet.
- Framework integrations are not bundled; applications or adapters choose the
  locale and response format.
- Rust runtime reflection is not required from users. Field access is generated
  by `#[derive(Validate)]` and kept behind the public validation DSL.

## Error Reporting

All public validation entry points return `Error`. Validation failures use
`Error::Failed(Vec<FieldError>)`; configuration errors use other `Error`
variants such as `UnknownRule`, `InvalidSchema`, or `InvalidData`.

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

Each `FieldError` exposes:

- `namespace`
- `struct_namespace`
- `field`
- `struct_field`
- `kind`
- `rule`
- `reason`
- `params`

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo package --manifest-path derive/Cargo.toml --allow-dirty
cargo publish --manifest-path derive/Cargo.toml --dry-run --allow-dirty
cargo package --list --allow-dirty
```

Root package verification requires `validator-derive` with the matching version
to be available in the target registry, so publish or provide
`validator-derive` first. The crates.io package name `validator` is already
occupied; publishing to crates.io needs a separate package naming decision, or
use a private registry that owns the `validator` name.

## Examples

```sh
cargo run --example v1
```
