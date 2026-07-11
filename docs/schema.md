# Schema Validation

[中文](schema.zh-CN.md)

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
unsigned, and `float` requires a floating-point JSON/YAML value. An integer
token is rejected for `type: float`, and a floating-point token is rejected for
`type: int`.

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
