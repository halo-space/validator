# Architecture

[中文](architecture.zh-CN.md)

## Design Note: Reflection

This project is under active initial development and does not preserve legacy
APIs or configuration spellings. When a type name, rule name, or API boundary
is incorrect, the implementation replaces it directly instead of adding
compatibility aliases, deprecated wrappers, or fallback parsing. A proposed
compatibility patch is treated as a signal to re-examine the underlying
architecture first.

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
