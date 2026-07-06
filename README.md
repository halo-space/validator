# validator

`validator` is a Rust 2024 validation library inspired by
`go-playground/validator`, but built around Rust-friendly derive macros,
typed value dispatch, and an extensible rule registry.

The current implementation focuses on code-level validation:

- `#[derive(Validate)]` for struct validation.
- `Validator::new().validate(&value)?` as the default entry point.
- Chainable runtime configuration with `Validator::new().alias(...)? .rule(...)?`.
- Built-in rules for required values, size checks, comparisons, strings, formats,
  choices, colors, and URLs.
- Consistent error reporting through `Errors`, `FieldError`, `Namespace`, and
  `Args`.

Runtime schema validation, nested struct traversal, collection `dive(...)`, and
i18n are planned separately and are not part of this first cut.

## Requirements

- Rust edition: `2024`
- Minimum supported rustc: `1.96`

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

    let errors = Validator::new()
        .alias("username", "required,length(min=3,max=20)")?
        .validate(&account)
        .unwrap_err();

    assert_eq!(errors.len(), 1);
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

## Built-In Rules

Current built-in rules:

- Presence: `required`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `gt`, `gte`, `lt`, `lte`
- Choice: `oneof`
- String: `contains`, `startswith`, `endswith`, `alpha`, `alphanum`,
  `numeric`, `number`, `lowercase`, `uppercase`, `boolean`
- Format: `email`, `regex`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`, `cmyk`
- Network: `url`
- Alias: `iscolor`

Comparison and size rules dispatch by field type:

- Strings use character count.
- Vectors, arrays, slices, and maps use item count.
- Signed integers, unsigned integers, and floats use their own numeric families.
- `Option::None` skips non-`required` rules and fails `required`.

## Errors

Validation failures return `Errors`, which contains one `FieldError` per failed
field rule.

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

Each `FieldError` exposes:

- `namespace`
- `struct_namespace`
- `field`
- `struct_field`
- `rule`
- `actual_rule`
- `args`

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
