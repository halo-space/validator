# validator

`validator` is a Rust 2024 validation library built around Rust-friendly derive
macros, typed value dispatch, dynamic Schema validation, and an extensible rule
registry.

[中文文档](README.zh-CN.md)

The current implementation is centered on code-level validation and also supports dynamic Schema validation:

- `#[derive(Validate)]` for struct validation.
- `Validator::new().validate(&value)?` as the default entry point.
- Selective derive validation through `partial(...)`, `except(...)`, and
  positive `filter(...)` predicates.
- `Validator::new().value(&value, "rules")?` for direct single-value validation.
- Chainable runtime configuration with `Validator::new().alias(...)? .rule(...)?`.
- Built-in rules for required values, size checks, comparisons, strings,
  formats, network identifiers, choices, and colors.
- Explicit nested struct validation with `#[validate(nested)]`.
- Collection validation with `dive(...)` for Vec, arrays, slices, and map
  key/value pairs.
- Cross-field validation with `eq_field`, `ne_field`, `gt_field`, `gte_field`,
  `lt_field`, and `lte_field`.
- Conditional field validation with `required_if`, `required_unless`,
  `skip_unless`, `required_with`, `required_with_all`, `required_without`,
  `required_without_all`, and `excluded_*` rules.
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
- Minimum supported rustc: `1.97`

## Git Dependency

This project is currently intended to be consumed from Git, not crates.io.

```toml
[dependencies]
validator = { git = "ssh://git@github-halo/halo-space/validator.git" }
```

Use the Git URL that is reachable from your environment. Pin `rev`, `tag`, or
`branch` in applications that need reproducible builds.

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
Generated access remains selective: direct fields and complete nested targets
are emitted only when a validation attribute actually references them.

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

## Selective Validation

Derived models can validate only part of a value without duplicating their
rules:

```rust
let validator = Validator::new();

validator.partial(&user, ["name", "profile.email"])?;
validator.except(&user, ["password_hash"])?;
validator.filter(&user, |namespace| {
    matches!(namespace.as_str(), "profile" | "profile.email")
})?;
```

Selectors use relative Rust struct namespaces, without the root type name.
Nested fields use dot paths, collection entries use concrete namespaces such as
`items[0].email`, and map entries use the same quoted-key form returned in
errors. Selecting a parent includes its complete subtree. Excluding a parent
skips its complete subtree; excluding one child keeps its siblings.

The `filter` callback has positive semantics: `true` validates that field or
entry, while `false` skips it and its descendants. A callback that wants a
nested field must also return `true` for its ancestors. The callback may be
called more than once for a namespace and should not rely on call count.
Struct-level errors are filtered through the same selection. An empty
`partial` selection validates nothing; an empty `except` selection is the same
as full validation. Every non-empty selector must match a declared field or
collection entry; otherwise validation returns `Error::UnknownField`. Wildcard
paths are not supported. Selective validation is generated by `#[derive(Validate)]`;
a hand-written `Validate` implementation supports full validation only.

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
Runtime expressions are strict: empty rules, empty alternatives, unbalanced
parentheses, unclosed quotes, and dangling escapes return
`Error::InvalidRuleExpression`. Inside quoted parameters, `\\` escapes the next
character, so use `\\\\` when the parameter itself needs one backslash.

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
Element rule parameters are preflighted from their declared `Value` kind before
iteration, so an empty collection cannot hide an invalid rule configuration.

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
Floating-point uniqueness follows equality semantics: repeated NaN values are
not duplicates, while `0.0` and `-0.0` are duplicates. A known scalar Kind is
rejected during parameter preflight, so `omitempty` cannot hide invalid use of
`unique`.

Struct collections can be unique by one or more element-relative fields:

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

`unique = "email"` remains the one-field shorthand for
`unique = ["email"]`. The bound error parameter is always
`fields = ["email"]` or the declared compound list. Paths are relative to each
element, and only terminal fields need to implement `Value`. Duplicate errors
stay on `Team.members`. Map projection, projection inside `dive(...)`, and
native `Validator::value(&members, "unique=email")` do not provide element
field context; the direct entry returns `MissingFieldContext`.

Native collections participate in rules outside `dive(...)` through `Value`.
Built-in scalar elements such as strings and numbers already implement it. A
custom element type must implement `Value` before its collection can use rules
that read the elements themselves, such as no-parameter `unique`:

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

This bound comes from the current static `Value` dispatch model; no runtime
reflection or hidden fallback is used. A collection field that only uses
`dive(nested)` is independent from this bound: its elements only need to
implement `Validate`.

`Value::declared_kind()` defaults to `None` for values whose kind is genuinely
dynamic, such as JSON. Custom statically typed values should return their kind
when they can appear inside `Option` or `dive(...)`; this lets parameter
preflight work even when no concrete value is present.

## Cross-Field Validation

Use `*_field` rules when one field should be compared with a sibling or a
downward nested field.

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

Supported rules are `eq_field`, `ne_field`, `gt_field`, `gte_field`,
`lt_field`, and `lte_field`. A one-segment target remains a same-level sibling;
a dotted target is relative to the struct containing the validated field and
walks downward. Every `Option<T>` segment is borrowed automatically. If any
target segment is `None`, the target is missing and every comparison rule,
including `ne_field`, fails. A current `Option::None` still skips the
cross-field rule unless `required` is also present.

Only the terminal target must implement `Value`; intermediate structs do not
need `Value` or `Validate`. Reading a path does not run nested validation:
`#[validate(nested)]` remains the only derive instruction that validates the
nested struct itself. Paths accept canonical dot-separated Rust field names
only. They do not support parent/root traversal, arrays, map keys, or
wildcards. A raw field such as `r#type` is written as `"type"`.

Equality and ordering are separate: string equality compares content, while
ordered string rules compare Unicode character count. Collections compare item
count, numeric and `SystemTime` values use their own families, and bool supports
only `eq_field` / `ne_field`. Float NaN is unequal and unordered.

Conditional field rules continue to use same-level sibling fields:

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

Supported conditional rules are `required_if`, `required_unless`,
`skip_unless`, `required_with`, `required_with_all`, `required_without`,
`required_without_all`, `excluded_if`, `excluded_unless`, `excluded_with`,
`excluded_with_all`, `excluded_without`, and `excluded_without_all`. They are
available for derive and Schema validation. `*_if` / `*_unless` compare sibling
field values by type; `*_with` / `*_without` check whether sibling fields are
present and non-empty. The `_all` variants require all referenced fields to
match the condition; the non-`_all` field-list variants trigger when any
referenced field matches the condition. `excluded_*` rules require the current
field to be empty when their condition is triggered. `skip_unless` follows Go's
current behavior: when all keyed conditions match, the current field is
required; otherwise the rule passes.
Use the quoted string `"null"` when an `*_if` or `*_unless` condition should
match a missing or `Option::None` sibling, including typed numeric fields.

## SystemTime Validation

Native time validation intentionally supports only `std::time::SystemTime`, the
standard-library time point closest to Go's `time.Time` use case.

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

For `SystemTime`, no-parameter `lt`, `lte`, `gt`, and `gte` compare against one
captured `now` per validation call. Literal parameters such as
`#[validate(gt = "2026-07-08T00:00:00Z")]` are rejected as configuration
errors because `SystemTime` has no single built-in string format. `eq` and `ne`
do not compare with the current time; use `eq_field` or `ne_field` for time
equality.

The `datetime` rule remains string format validation. Dynamic Schema validation
does not have a native `type: time`; use `type: string` plus `datetime` for
timestamp text.

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

Schema single-target field rules use the same relative dotted syntax:

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

Every intermediate Schema segment must be a declared `object`; arrays are not
traversed by this feature. Missing or null runtime segments produce a missing
target while preserving the terminal field's declared `Kind`. Schema aliases
may contain dotted field rules because alias expansion and path validation both
run while the Schema tree is compiled. A one-segment target is an exact
serialized field name, so names such as `source-url` remain valid. A target
containing `.` always means a nested path; Schema compilation rejects a scope
that also declares a literal dotted field with the same name rather than
choosing one meaning by precedence.

Schema resources are strict. The top level accepts only `fields`, and each
field definition accepts only `type`, `rules`, and `fields`. The only type names
are `string`, `boolean`, `int`, `uint`, `float`, `array`, and `object`.
Nested `fields` are valid only for `object` and `array` types.
The presence of the key is structural even when it is empty: `fields: {}`
infers `object` when `type` is omitted, is rejected for scalar types, and makes
every item of an array an object. An array without `fields` has no item object
constraint.
Unknown keys and unknown types return `Error::InvalidSchema`. Parameters that
violate a Rule `Signature` return `Error::InvalidRuleExpression` when the
Schema is compiled.
Semantic parameters are also preflighted when the cached Schema tree is built,
before root or field input type checks can short-circuit rule execution.

Schema validation is JSON/YAML-data oriented. It supports `datetime` as a string
rule, but does not support native `SystemTime` values or `type: time`.
Numeric rules follow the declared Schema family: `int` is signed, `uint` is
unsigned, and `float` is floating-point even when the JSON token is written as
an integer.

Schema type names and validation rule names belong to different namespaces.
`type: float` declares the field's data type. The `number` rule is a predicate:
it accepts native numeric values and strings containing ASCII digits only, such
as `"12345"`; it rejects signs and decimal points. The `numeric` rule also
accepts native numeric values, but its string form may contain a leading sign
and a decimal fraction, such as `"-12.5"`. Therefore `number` is valid under
`rules`, but is not a valid Schema `type`.

For `type: array`, `fields` describes object elements and supports field
uniqueness directly:

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

Element field errors use namespaces such as `users[0].email`. Non-object items
including `null` produce a `type` error at a namespace such as `users[0]`.
`unique: email` is the scalar shorthand for a one-item fields list. If a value
projected by a single or compound unique rule violates a child field type,
unique skips that complete item key and the child reports its indexed `type`
error; malformed input is not reclassified as invalid rule configuration.

If the data already implements `serde::Serialize`, use `validate_serde(...)`.
Schema field names follow the serialized data shape, including
`serde(rename)`, `serde(rename_all)`, `serde(skip_serializing_if)`, and
`serde(flatten)`.
The Schema is resolved and compiled before user serialization, so
`MissingSchema` or invalid Schema configuration cannot be hidden by a custom
serializer failure.

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

Locale resources are also strict: unknown keys return `Error::InvalidData`.
The language identifier key is only `locale`; `name` is not accepted as a
compatibility spelling. `Locale::locale()` returns the identifier of a
programmatically constructed locale.

## Built-In Rules

Current built-in rules:

- Required/Optional: `required`, `isdefault`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `eq`, `ne`, `eq_ignore_case`, `ne_ignore_case`, `gt`, `gte`,
  `lt`, `lte`
- Field-aware: `eq_field`, `ne_field`, `gt_field`, `gte_field`, `lt_field`,
  `lte_field`, `fieldcontains`, `fieldexcludes`, `required_if`,
  `required_unless`, `skip_unless`, `required_with`, `required_with_all`,
  `required_without`, `required_without_all`, `excluded_if`, `excluded_unless`,
  `excluded_with`, `excluded_with_all`, `excluded_without`,
  `excluded_without_all` for derive and Schema validation
- Collection: `unique`
- Choice: `oneof`, `oneofci`, `noneof`, `noneofci`
- String: `contains`, `containsany`, `containsrune`, `excludes`,
  `excludesall`, `excludesrune`, `startswith`, `endswith`, `startsnotwith`,
  `endsnotwith`, `ascii`, `printascii`, `multibyte`, `alpha`, `alphaspace`,
  `alphaunicode`, `alphanum`, `alphanumspace`, `alphanumunicode`, `numeric`,
  `number`, `lowercase`, `uppercase`, `boolean`

  Here `number` means an ASCII-digit predicate for strings, not a Schema type;
  `numeric` additionally accepts a sign and decimal fraction.
- Format: `email`, `regex`, `json`, `datetime`, `e164`, `base32`, `base64`,
  `base64url`, `base64rawurl`, `hexadecimal`, `url_encoded`, `html`,
  `html_encoded`, `jwt`, `mac`, `semver`, `origin`, `datauri`, `latitude`,
  `longitude`, `ssn`, `md4`, `md5`, `sha256`, `sha384`, `sha512`,
  `ripemd128`, `ripemd160`, `tiger128`, `tiger160`, `tiger192`, `eth_addr`,
  `mongodb`, `mongodb_connection_string`, `dns_rfc1035_label`, `cve`, `cron`,
  `ein`, `bic_iso_9362_2014`, `bic`, `isbn`, `isbn10`, `isbn13`, `issn`,
  `credit_card`, `luhn_checksum`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`,
  `cmyk`
- Network: `url`, `uri`, `http`, `https`, `ip`, `ipv4`, `ipv6`,
  `cidr`, `cidrv4`, `cidrv6`, `hostname`,
  `hostname_port`, `hostname_rfc1123`, `fqdn`, `port`, `uuid`, `uuid3`,
  `uuid4`, `uuid5`, `uuid_rfc4122`, `uuid3_rfc4122`, `uuid4_rfc4122`,
  `uuid5_rfc4122`, `ulid`, `tcp4`, `tcp6`, `tcp`, `udp4`, `udp6`, `udp`
- Alias: `iscolor`

Ordered comparison and size rules dispatch by field type:

- Strings use character count.
- Vectors, arrays, slices, and maps use item count.
- Signed integers, unsigned integers, and floats use their own numeric families.
- `std::time::SystemTime` supports no-parameter time comparison against a
  captured `now` and same-kind `*_field` comparison.
- `Option::None` skips non-`required` rules and fails `required`.

Equality rules compare string content instead of length. `length` rejects an
empty configuration and does not allow `exact` together with `min` or `max`;
`length` and `range` reject reversed bounds during parameter preflight.

Choice rules dispatch by field type for strings, signed integers, and unsigned integers.
URL and URI rules use structured parsers. `hostname` follows RFC952, while
`hostname_rfc1123` permits a leading digit and `fqdn` requires a non-numeric TLD.
`cidr` accepts IPv4 or IPv6 address-prefix notation, while `cidrv4` additionally
requires a canonical network address. `mac` accepts 6-, 8-, and 20-octet link
addresses, and lowercase `uuid4` / `uuid5` check both version and RFC variant.

## Current Limits

These are intentional limits in the current API surface:

- Parameterized `unique` supports single, compound, and nested element paths in
  Vec, arrays, slice references, and Schema object arrays; it does not support
  maps, native direct values, or use inside `dive(...)`.
- Nested field targets are relative and downward-only. Parent/root paths,
  collection indices, map keys, and conditional-rule paths are not supported.
- Derive code must spell element paths explicitly because a runtime alias cannot
  generate Rust field access. Schema aliases can contain parameterized unique
  expressions because Schema fields are available at runtime.
- Native collections using generic rules outside `dive(...)` require their
  element or map value type to implement `Value`; pure `dive(nested)` only
  requires `Validate`.
- Native time validation is limited to `std::time::SystemTime`; `Duration`,
  `chrono`, and `time` crate values are not built in.
- `country_code` and related country aliases are not built in yet.
- Framework integrations are not bundled; applications or adapters choose the
  locale and response format.
- Rust runtime reflection is not required from users. Field access is generated
  by `#[derive(Validate)]` and kept behind the public validation DSL.

## Error Reporting

All public validation entry points return `Error`. Validation failures use
`Error::Failed(Vec<FieldError>)`; configuration errors use other `Error`
variants for unknown rules, duplicate names, invalid parameters, missing field
context, recursive aliases, invalid schemas, or invalid data.
Configuration preflight always completes before data-dependent rule flow, so
invalid parameters are returned even for absent, empty, or otherwise skipped
values.

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
cargo bench --bench validation
cargo package --manifest-path derive/Cargo.toml --allow-dirty
```

The benchmark suite measures derive success and failure, selective validation,
warm and cold direct expressions, warm and cold Schema execution,
`validate_serde`, collection dive, and compound unique projection. Use
`cargo bench --bench validation -- --quick` for a short local verification run.

The current distribution path is Git dependency usage. Registry publishing is
out of scope for now. The root `validator` crate is intentionally not packaged
for crates.io in this phase; only the technical `validator-derive` package is
checked with `cargo package`.

## Examples

```sh
cargo run --example v1
```
