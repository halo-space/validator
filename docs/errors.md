# Error Model

[中文](errors.zh-CN.md)

## Error Reporting

All public validation entry points return `Error`. Validation failures use
`Error::Failed(Vec<FieldError>)`; configuration errors use other `Error`
variants for unknown rules, duplicate names, invalid parameters, missing field
context, recursive aliases, invalid schemas, or invalid data.
Configuration preflight always completes before data-dependent rule flow, so
invalid parameters are returned even for absent, empty, or otherwise skipped
values.

| Variant | Meaning |
| --- | --- |
| `Failed` | One or more fields failed validation. |
| `UnknownRule` | A runtime expression or alias references an unregistered rule. |
| `UnknownField` | A selector or struct-level error references an undeclared root field. |
| `InvalidRuleExpression` | Rule syntax, parameter shape, or parameter semantics are invalid. |
| `InvalidRuleName`, `InvalidAliasName` | A registered name contains unsupported characters. |
| `DuplicateName` | A rule, alias, or reserved control name is already registered. |
| `MissingFieldContext` | A field-aware rule was used by direct value validation. |
| `RecursiveAlias` | Alias expansion encountered a cycle. |
| `MissingSchema` | Schema validation was requested without a configured schema. |
| `InvalidSchema` | A Schema resource is malformed or internally inconsistent. |
| `InvalidData` | Serialization or locale resource parsing failed. |

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
