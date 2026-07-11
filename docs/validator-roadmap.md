# Validator Roadmap

> 本文保留项目的设计决策、阶段计划和命名约束，其中部分阶段清单与目录示例
> 记录的是当时的计划，不作为当前 API 参考。当前用法以
> [README](../README.zh-CN.md) 和 `docs/` 下的主题文档为准。

## 目标

做一个对外只暴露 `validator` 的 Rust 校验库，使用体验尽量向 `go-playground/validator` 靠拢：

- 使用上简单统一
- 内部结构清晰可扩展
- 代码层声明式校验作为主线
- YAML / JSON dynamic Schema 校验已经作为 V2 能力接入，复用同一套执行模型

## 总原则

- 对外只依赖一个 crate：`validator`
- 对内使用模块分层：`core` / `rules` / `i18n`
- `derive/` 只作为 Rust proc-macro 技术包存在
- 不复制 Go 的运行时反射实现
- V1 不引入 `std::any` / `TypeId` / `type_name` 作为核心分发机制
- 采用 Rust 风格：`derive` 负责采集元数据，`core` 负责统一执行
- 所有规则最终落到同一个内部规则模型

## V1 范围与 V2 扩展

V1 主线是代码层声明式校验；V2 已引入 YAML / JSON dynamic Schema 校验，复用同一套 `Validator`、规则注册表、alias、`Value` 分派、`Error` 和 `Namespace` 模型。

最小使用示例：

```rust
use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required, length(min = 3, max = 20))]
    name: String,

    #[validate(email)]
    email: String,
}

Validator::new().validate(&user)?;
```

自定义 rule / alias 示例：

```rust
Validator::new()
    .alias("username", "required,length(min=3,max=20)")?
    .rule("slug", Slug)?
    .validate(&user)?;
```

## V1 API Spec 与命名约束

### 对外 API

V1 最终对外主路径：

```rust
impl Validator {
    pub fn new() -> Self;

    pub fn validate<T: Validate>(&self, value: &T) -> std::result::Result<(), Error>;

    pub fn rule<R>(
        self,
        name: impl Into<String>,
        rule: R,
    ) -> std::result::Result<Self, Error>
    where
        R: Rule + Send + Sync + 'static;

    pub fn alias(
        self,
        name: impl Into<String>,
        expr: impl AsRef<str>,
    ) -> std::result::Result<Self, Error>;
}
```

约束：

- `Validator::new().validate(&user)?` 必须可用。
- `Validator::new().alias(...)? .rule(...)? .validate(&user)?` 必须可用，这个写法是公开 API 约束，不能被实现方案影响。
- `rule(...)` 和 `alias(...)` 是 `Validator` 本体上的 by-value 链式方法，返回 `std::result::Result<Self, Error>`。
- `validate(...)` 使用 `&self`，返回 `std::result::Result<(), Error>`。
- 不定义 `ValidatorResult`、`ValidationResult`、`RuleResult` 这类别名。
- 不使用 `register_rule`、`register_alias` 作为公开方法名，对外统一叫 `rule`、`alias`。
- 不引入会改变公开写法的 builder 形态，例如不能把主路径改成 `Validator::builder().alias(...).build()?`。
- 链式示例中的 `?` 只需要调用方错误类型能承载 `validator::Error`，例如应用层自定义错误或 `Box<dyn std::error::Error>`。

### 错误模型

V1 只保留两类公开错误相关类型：

```rust
pub enum Error;
pub struct FieldError;
```

语义：

- `Error` 是 validator 统一公开错误类型，覆盖配置错误和校验失败。
- `Error::Failed(Vec<FieldError>)` 表示一次数据校验失败的结果，内部包含多个 `FieldError`。
- `FieldError` 表示单个字段上的一条规则失败。
- 其他 `Error` 变体表示 validator API 使用错误或配置错误，例如非法 rule 名、非法 alias、未知 rule、未知 alias。

约束：

- 不使用 `ValidationError` / `ValidationErrors` 命名。
- 不使用 `Report` / `Violation` 命名。
- 不定义单独的公开 `Errors` / `ValueError` / `MapError` 包装类型。
- `validate(...)` 遇到多个字段失败时应尽量收集后统一返回 `Error::Failed(Vec<FieldError>)`，不默认遇到第一条错误就中断。
- `Error` 需要实现 `std::error::Error` 和 `Display`，方便用户直接 `?`。

### Namespace

错误定位统一使用 `Namespace` 概念，不使用 `FieldPath` / `Path` 作为对外类型名。

```rust
pub struct Namespace(String);
```

`FieldError` 至少应暴露：

```rust
impl FieldError {
    pub fn namespace(&self) -> &Namespace;
    pub fn struct_namespace(&self) -> &Namespace;
    pub fn field(&self) -> &str;
    pub fn struct_field(&self) -> &str;
    pub fn kind(&self) -> Kind;
    pub fn rule(&self) -> &str;
    pub fn reason(&self) -> &str;
    pub fn params(&self) -> &Params;
}
```

含义：

- `namespace` 优先使用外部字段名，例如未来支持 `serde(rename = "...")` 后的名字。
- `struct_namespace` 使用 Rust struct 原始字段名。
- `field` 是当前字段名，优先使用外部字段名。
- `struct_field` 是当前 Rust struct 原始字段名。
- `kind` 是失败字段的语义类型，用于 i18n 动态文案和后续类型相关扩展。
- `rule` 是用户写的规则名；如果失败来自 alias，优先保留 alias 名。
- `reason` 是实际失败的底层规则名；普通规则下等于 `rule`，alias 场景下是展开后真正失败的规则。
- `params` 是规则参数集合，例如 `length(min = 3, max = 20)` 中的 `min` 和 `max`。

namespace 示例：

```text
User.name
User.profile.email
User.source_urls[1]
User.items[0].title
```

### 文件与内部命名

文件命名：

```text
src/core.rs
src/core/error.rs
src/core/params.rs
src/core/field.rs
src/core/namespace.rs
src/core/registry.rs

src/rules.rs
src/rules/alias.rs
src/rules/required.rs
src/rules/required/base.rs
src/rules/size.rs
src/rules/size/min.rs
src/rules/compare.rs
src/rules/compare/gte.rs
src/rules/string.rs
src/rules/string/contains.rs
src/rules/format.rs
src/rules/format/email.rs
src/rules/network.rs
src/rules/network/url.rs
derive/src/lib.rs
```

`registry.rs` 内部命名：

```rust
pub(crate) struct Registry;
```

约束：

- rule 与 alias 使用同一名称空间和同一个 `Registry`，不拆成 `Rules` / `Aliases`、`RuleRegistry` / `AliasRegistry`。
- 不使用 `baked_in.rs`、`bakedInValidators`、`bakedInAliases` 作为 Rust 版命名。
- 不使用 `default_rules.rs` / `default_aliases.rs`，规则文件直接叫 `rules.rs` / `alias.rs`。
- 规则实现按语义分类，不在 `src/rules/` 下无限平铺。
- 分类入口使用 `src/rules/string.rs` 这类文件，不使用 `mod.rs`。
- 对外保留 `pub trait Rule`，因此 registry 内部不要再定义同名 `struct Rule`。

### 自定义规则命名约束

自定义规则 trait 对外保留：

```rust
pub trait Rule {
    fn check(&self, field: &Field<'_>) -> Result<bool, Error>;
}
```

约束：

- 不使用 `RuleInput` / `RuleResult` 命名。
- 规则执行上下文命名为 `Field<'_>`，表示“当前正在校验的字段”。
- 规则参数命名为 `Params`，不使用过于泛化的 `Input`。
- 规则方法名用 `check`，避免和 `Validator::validate` 混淆。
- `Rule::check` 返回 `Result<bool, Error>`：`Ok(false)` 表示数据未通过校验，`Err(Error)` 表示规则配置或执行阶段错误。
- 自定义 rule 不自己拼 `FieldError`。
- `FieldError` 由 core executor 统一生成，保证 namespace、rule、reason、params 等字段一致。

### 字段类型分派原则

Go 版 `isLte` / `isGte` / `required` 这类规则的关键不是反射本身，而是：

- 先看字段实际类型。
- 再按这个类型读取值。
- 同一个规则名在不同字段类型上有不同语义。

Rust 版不照搬 `reflect.Kind()`，但保留这个模型。`Value` 应提供字段类型分类和值读取方法，规则按类型分派。

V1 不依赖 Rust 运行时反射能力。validator 通过 `Value` trait 暴露校验所需的语义信息，通过 `Kind` 完成规则分发。`std::any` / `TypeId` / `type_name` 暂不进入核心 API，后续仅在确实需要自定义类型识别或 Rust 反射能力成熟后再评估引入。

建议核心类型：

```rust
pub enum Kind {
    String,
    Bool,
    Int(IntKind),
    Uint(UintKind),
    Float(FloatKind),
    Vec,
    Array,
    Slice,
    Map,
    Option,
    Time,
    Other,
}

pub enum IntKind {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
}

pub enum UintKind {
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

pub enum FloatKind {
    F32,
    F64,
}

pub enum Number {
    Int(i128),
    Uint(u128),
    Float(f64),
}
```

命名约束：

- 不使用 `as_str` / `as_f64` 这类转换式命名。
- `Value` 不使用 `present` / `absent` / `text` 这类额外概念。
- `Value` 的字符串读取叫 `string()`；规则参数结构中的单值变体使用 `Param::Text`，两者职责不同。
- `is_empty()` 只作为 `len()` 的 Rust 标准配套方法，不引入新的存在性概念。
- 数字读取按 `int()` / `uint()` / `float()` 分开，不把整数默认压成 `f64`。
- `Kind` 必须保留具体数字类型，例如 `IntKind::I32`、`UintKind::U8`、`FloatKind::F32`。
- optional skip 可以叫 `is_none()`，对应 Rust 的 `Option::None`。
- `Option::Some(_)` 满足 `required`；其他取值方法委托内部值。

比较类规则应按 `Kind` 分支：

- `String` 使用字符长度。
- `Vec` / `Array` / `Slice` / `Map` 使用元素数量。
- `Int` / `Uint` / `Float` 使用对应数值。
- `Time` 后续单独设计；核心库只支持 Rust 标准库 `std::time` 范围内的原生时间类型，不内置 `chrono`、`time` 等第三方包支持。

参数比较可以按数字族归一化：

- `i8/i16/i32/i64/i128/isize` 读取为 `i128`。
- `u8/u16/u32/u64/u128/usize` 读取为 `u128`。
- `f32/f64` 读取为 `f64`，但参数解析需要区分 `FloatKind::F32` 和 `FloatKind::F64`。

字段间比较后续要保留具体类型一致性：

- `i32` 和 `i64` 不应默认视为同一种字段类型。
- `u8` 和 `u64` 不应默认视为同一种字段类型。
- `f32` 和 `f64` 不应默认视为同一种字段类型。

这对应 Go 版字段间比较里的 `currentKind != kind` 判断。

这组规则共享底层比较分派，比较分派放在 `src/rules/compare.rs`，不放在 `src/rules.rs` 顶层：

```text
lt / lte / gt / gte / min / max / length / range
```

`required` 不走比较 helper，由每个 `Value` 实现自己定义是否满足必填。

### 第三方依赖原则

可接受依赖：

```toml
thiserror = "2"
regex = "1"
```

原则：

- 库内部错误类型用 `thiserror`，不在公开 API 中使用 `anyhow`。
- `regex` 规则使用 `regex` crate，不手写正则引擎。
- 后续按规则需要再引入 `url`、`uuid`、`email_address` 等专用 crate。
- 时间类型是例外：只支持 Rust 标准库 `std::time`，不内置 `chrono`、`time` 等第三方时间类型，避免依赖范围无穷扩张。
- `bon` 是候选依赖，不是 V1 强制依赖；只有在不改变公开 API 的前提下，才考虑用于内部复杂 builder 或辅助配置对象。

### `bon` 使用边界

参考资料：

- [`elastio/bon`](https://github.com/elastio/bon)
- [`bon` Guide](https://bon-rs.com/guide/overview)
- [`bon` Fallible Builders](https://bon-rs.com/guide/patterns/fallible-builders)
- [`bon` Typestate API](https://bon-rs.com/guide/typestate-api)

可以使用：

- 如果后续出现类似 `#[derive(Builder)]` 适合表达的复杂配置对象，可以考虑用 `bon` 生成 builder。
- 如果 `bon` 能在不影响公开 API 的情况下简化内部配置构建，可以使用。

硬约束：

- `Validator::new().validate(&user)?` 是最小成功路径。
- `Validator::new().alias(...)? .rule(...)? .validate(&user)?` 是自定义配置路径。
- 如果 `bon` 的生成方式要求把公开 API 改成 `Validator::builder().alias(...).build()?`，则不用 `bon`，我们自己实现 `Validator` 链式方法。
- `alias(...)` / `rule(...)` 这种主链路方法本身很简单，优先手写，避免为了工具牺牲 API。
- 不为了使用 `bon` 而新增用户可见的 `ValidatorBuilder` 类型。
- validator 需要支持运行时 rule / alias 注册，因此不能把全部规则能力做成编译期 typestate。
- derive 只负责生成校验元数据和调用代码，不把运行时规则注册烤进类型系统。

## V2 Dynamic Schema

配置文件驱动校验作为 V2 dynamic Schema 能力提供。

当前 API：

```rust
let schema = Schema::from_yaml(yaml)?;
Validator::with_schema(schema).validate_map(&payload)?;
```

对应 JSON：

```rust
let schema = Schema::from_json(json)?;
Validator::with_schema(schema).validate_map(&payload)?;
```

说明：

- `Schema::from_yaml` / `Schema::from_json` 负责加载配置
- `Validator::with_schema` 负责绑定 schema，校验时按 validator generation 编译并缓存 Schema `Tree`
- `validate_map` 当前接收 `serde_json::Value` object
- `validate_serde` 已支持 `serde::Serialize` 对象校验；它先取得并编译 Schema Tree，再序列化成 `serde_json::Value`，最后与 `validate_map` 复用同一个内部数据校验函数
- `serde_yaml::Value` 直传可以作为后续增强，但不新增第二套 Schema 执行模型

## Roadmap

### Phase 0 - 工程骨架

目标：

- 建立根 crate：`validator`
- 用 `src/core.rs`、`src/rules.rs`、`src/i18n.rs` 做模块入口
- 不使用 `mod.rs`
- 只保留 `derive/` 作为 `#[derive(Validate)]` 的 proc-macro 技术包

交付：

- `src/lib.rs`
- `src/core.rs` 与 `src/core/`
- `src/rules.rs` 与 `src/rules/`
- `src/i18n.rs`: i18n 公开入口与类型重导出
- `src/i18n/locale.rs`: Locale 加载、合并、选择和 Message 渲染上下文
- `src/i18n/template.rs`: Template 与单次扫描占位符渲染
- `src/i18n/zh_cn.rs`: 内置 `zh-CN` 资源
- `src/i18n/en.rs`: 内置 `en` 资源
- `derive/`
- 基础 CI / test / clippy / fmt

完成标准：

- `cargo test` 可运行
- `validator` 能正确 `pub use` 核心类型和 derive 宏

### Phase 1 - Core 数据模型

目标：

- 定义统一执行模型
- 为后续 derive 与 V2 schema 留同一条主线

公开核心类型：

- `Validator`
- `Error`
- `FieldError`
- `Namespace`
- `Field`
- `Params`
- `Rule`
- `Value`

内部执行模型类型：

- `Spec`
- `Expr`
- `Group`
- `FieldTarget`
- `Registry`

完成标准：

- 错误结构稳定
- namespace 表达稳定
- 规则和 alias 能按名称查找并执行

### Phase 2 - Derive 与代码层最小闭环

目标：

- 打通 `#[derive(Validate)]`
- 支持字段级校验
- 跑通 `Validator::new().validate(&value)`

第一版公开 API：

```rust
pub trait Validate {
    fn validate(&self, validator: &Validator) -> std::result::Result<(), Error>;
}

impl Validator {
    pub fn new() -> Self;
    pub fn validate<T: Validate>(&self, value: &T) -> std::result::Result<(), Error>;
}
```

最小闭环属性：

- `#[validate(required)]`
- `#[validate(length(min = ..., max = ...))]`

完成标准：

- 至少支持 `String`、数字、`Option<T>`
- derive 生成代码清晰可读
- namespace 和错误收集可用
- `min` / `max` 在 Phase 3 作为常用规则补齐

### Phase 3 - 常用内置规则对齐 Go

目标：

- 先覆盖最常用、最有感知的一批规则

V1 常用优先规则：

这一节是 V1 常用规则目标清单，当前已实现规则以 README 的“当前内置规则”和 `src/rules.rs` 注册表为准。已经实现的规则要及时同步到 README、examples 和本 roadmap 的“已完成”列表，避免出现“代码已完成但文档仍像待办”的状态。

- `required`
- `omitempty`
- `length`
- `min`
- `max`
- `range`
- `email`
- `regex`
- `json`
- `datetime`
- `url`
- `uri`
- `http`
- `https`
- `ip`
- `ipv4`
- `ipv6`
- `cidr`
- `cidrv4`
- `cidrv6`
- `hostname`
- `hostname_rfc1123`
- `fqdn`
- `port`
- `uuid`
- `uuid3`
- `uuid4`
- `uuid5`
- `ulid`
- `hexcolor`
- `rgb`
- `rgba`
- `hsl`
- `hsla`
- `cmyk`
- `oneof`
- `noneof`
- `eq`
- `ne`
- `gt`
- `gte`
- `lt`
- `lte`
- `contains`
- `containsany`
- `startswith`
- `endswith`
- `ascii`
- `alpha`
- `alphanum`
- `numeric`
- `number`
- `lowercase`
- `uppercase`
- `boolean`
- `unique`

V1 常用规则已收口：

- `country_code` alias 不放入 core；这类标准数据维护留给扩展 crate。

说明：

- 命名可以保持 Rust 风格，但语义尽量和 Go 对齐
- 先做“常用正确”，不要一开始追求 README 那种超大全集
- `range` 是 Rust 侧便利用法，内部可以展开成 `min + max`，Go 版没有同名 tag
- `regex` 是 Rust 侧需要的常用规则，Go 版通过内置具体格式规则和自定义 validator 覆盖类似场景
- Rust attribute 主推 `length(...)`，不主推 Go 的 `len` 名称。
- 不为 Go tag 名称额外保留兼容层；如果后续需要兼容，应作为明确设计变更讨论。

完成标准：

- 有稳定单元测试
- 对应 README 示例可以跑通

### Phase 3.1 - Go 版规则对照 TODO

来源：

- Go 参考实现：[`go-playground/validator`](https://github.com/go-playground/validator)
- Go 规则来源：`baked_in.go` 中 `bakedInValidators`
- Go alias 来源：`baked_in.go` 中 `bakedInAliases`

术语说明：

- `bakedInValidators` 是 Go 版内部的“默认内置规则注册表”，也就是 `required`、`email`、`min`、`max` 这些规则名到具体校验函数的映射。
- `bakedInAliases` 是 Go 版内部的“默认规则别名表”，例如 `iscolor` 展开成 `hexcolor|rgb|rgba|hsl|hsla|cmyk`。
- 这里的规则清单只是用来防遗漏，不是未来 Rust 代码里的正式命名。

Rust 版命名约定：

- 不使用 `baked_in` 作为文件名或模块名。
- 默认规则放在 `src/rules.rs` 或 `src/rules/` 下语义明确的文件中。
- 默认 alias 放在 `src/rules/alias.rs`。
- 规则和 alias 的内部注册表放在 `src/core/registry.rs`，共享单一内部类型 `Registry`。
- 对外规则注册方法叫 `rule`，alias 注册方法叫 `alias`，避免使用 Go 版历史命名。

V1 完整版目标规则：

这一节用于对照 Go 版能力，避免后续遗漏；它不是当前已实现清单。

- 必填/可选: `required`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`
- String basics: `alpha`, `alphanum`, `ascii`, `contains`, `containsany`, `startswith`, `endswith`, `lowercase`, `uppercase`, `boolean`, `numeric`, `number`
- Format basics: `email`, `url`, `http`, `https`, `uri`, `uuid`, `uuid3`, `uuid4`, `uuid5`, `ulid`, `json`, `datetime`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`, `cmyk`
- Network basics: `ip`, `ipv4`, `ipv6`, `cidr`, `cidrv4`, `cidrv6`, `hostname`, `hostname_rfc1123`, `fqdn`, `port`
- Enum / membership: `oneof`, `noneof`
- Collection: `unique`
- Rust-specific addition: `regex`, `range`

Go tag 对照：

- 暂不提供兼容映射，避免公开 API 和规则命名出现双轨。

V1 结构能力必须覆盖但不一定作为普通 rule 实现：

- `nested`
- `dive`
- `dive(keys(...), values(...))`
- `Option<T>` required semantics
- struct-level validate function
- namespace error reporting

V1 跨字段规则：

- `eq_field`
- `ne_field`
- `gt_field`
- `gte_field`
- `lt_field`
- `lte_field`

单段目标表示同级字段；单目标 `*_field` 规则同时支持相对当前结构体向下的点分目标，例如 `profile.contact.email`。derive 只为属性实际引用的完整路径生成借用访问，中间结构不要求实现 `Value` / `Validate`，路径访问也不会替代显式 `nested`。Schema 单段目标按序列化字段名精确匹配，点分目标逐段确认中间节点为 `object`；同层的同名字面点分字段会在编译时作为歧义配置拒绝。运行时复用同一 Rule 与 `Field::sibling(...)` 语义。

Go tag 对照，不是兼容层：

- Go: `eqfield` / Rust 正式命名: `eq_field`
- Go: `nefield` / Rust 正式命名: `ne_field`
- Go: `gtfield` / Rust 正式命名: `gt_field`
- Go: `gtefield` / Rust 正式命名: `gte_field`
- Go: `ltfield` / Rust 正式命名: `lt_field`
- Go: `ltefield` / Rust 正式命名: `lte_field`

这些只是命名对照，不表示自动解析 Go tag，也不提供兼容 alias。Rust 版正式使用 `*_field` 命名。

V1 alias：

- `iscolor`

V1 已完成扩展规则：

- Extended strings: `alphaspace`, `alphanumspace`, `alphaunicode`, `alphanumunicode`, `multibyte`, `printascii`, `containsrune`, `excludes`, `excludesall`, `excludesrune`, `startsnotwith`, `endsnotwith`, `eq_ignore_case`, `ne_ignore_case`
- Encodings / hashes: `base32`, `base64`, `base64url`, `base64rawurl`, `hexadecimal`, `md4`, `md5`, `sha256`, `sha384`, `sha512`, `ripemd128`, `ripemd160`, `tiger128`, `tiger160`, `tiger192`
- Web / document formats: `html`, `html_encoded`, `url_encoded`, `datauri`, `jwt`, `cron`, `semver`, `cve`
- Financial / identity formats: `credit_card`, `luhn_checksum`, `bic`, `bic_iso_9362_2014`, `ein`, `ssn`
- Database / blockchain identifiers without checksum: `eth_addr`, `mongodb`, `mongodb_connection_string`
- Geographic: `latitude`, `longitude`
- Network extended: `tcp4`, `tcp6`, `tcp`, `udp4`, `udp6`, `udp`, `mac`, `origin`, `dns_rfc1035_label`, `hostname_port`
- Identifiers / misc: `isbn`, `isbn10`, `isbn13`, `issn`, `e164`, `noneofci`, `oneofci`, `uuid_rfc4122`, `uuid3_rfc4122`, `uuid4_rfc4122`, `uuid5_rfc4122`
- Field-aware: `fieldcontains`, `fieldexcludes`, `required_if`, `required_unless`, `skip_unless`, `required_with`, `required_with_all`, `required_without`, `required_without_all`, `excluded_if`, `excluded_unless`, `excluded_with`, `excluded_with_all`, `excluded_without`, `excluded_without_all`
- Default value: `isdefault`

V1 暂缓但要保留语义对照：

- Go `eqcsfield` / `necsfield` / `gtcsfield` / `gtecsfield` / `ltcsfield` / `ltecsfield` 不注册兼容名称；对应的向下嵌套字段能力已经由现有 `*_field = "a.b"` 提供。
- Crypto / checksum identifiers: `btc_addr`, `btc_addr_bech32`, `eth_addr_checksum`
- Geographic / locale: `timezone`, `iso3166_1_alpha2`, `iso3166_1_alpha2_eu`, `iso3166_1_alpha3`, `iso3166_1_alpha3_eu`, `iso3166_1_alpha_numeric`, `iso3166_1_alpha_numeric_eu`, `iso3166_2`, `iso4217`, `iso4217_numeric`, `bcp47_language_tag`, `bcp47_strict_language_tag`, `postcode_iso3166_alpha2`, `postcode_iso3166_alpha2_field`
- Filesystem / MIME: `file`, `filepath`, `dir`, `dirpath`, `image`, `mimetype`
- Network extended: `unix_addr`, `uds_exists`, `urn_rfc2141`
- Identifiers / misc: `validateFn`, `spicedb`
- Alias: `country_code`, `eu_country_code`

Go 版完整内置规则清单：

```text
alpha, alphanum, alphanumspace, alphanumunicode, alphaspace, alphaunicode,
ascii, base32, base64, base64rawurl, base64url, bcp47_language_tag,
bcp47_strict_language_tag, bic, bic_iso_9362_2014, boolean, btc_addr,
btc_addr_bech32, cidr, cidrv4, cidrv6, cmyk, contains, containsany,
containsrune, credit_card, cron, cve, datauri, datetime, dir, dirpath,
dns_rfc1035_label, e164, ein, email, endsnotwith, endswith, eq,
eq_ignore_case, eqcsfield, eqfield, eth_addr, eth_addr_checksum, excluded_if,
excluded_unless, excluded_with, excluded_with_all, excluded_without,
excluded_without_all, excludes, excludesall, excludesrune, fieldcontains,
fieldexcludes, file, filepath, fqdn, gt, gtcsfield, gte, gtecsfield, gtefield,
gtfield, hexadecimal, hexcolor, hostname, hostname_port, hostname_rfc1123,
hsl, hsla, html, html_encoded, http, https, image, ip, ip4_addr,
ip6_addr, ip_addr, ipv4, ipv6, isbn, isbn10, isbn13, isdefault,
iso3166_1_alpha2, iso3166_1_alpha2_eu, iso3166_1_alpha3,
iso3166_1_alpha3_eu, iso3166_1_alpha_numeric,
iso3166_1_alpha_numeric_eu, iso3166_2, iso4217, iso4217_numeric, issn, json,
jwt, latitude, len, longitude, lowercase, lt, ltcsfield, lte, ltecsfield,
ltefield, ltfield, luhn_checksum, mac, max, md4, md5, mimetype, min, mongodb,
mongodb_connection_string, multibyte, ne, ne_ignore_case, necsfield, nefield,
noneof, noneofci, number, numeric, oneof, oneofci, origin, port,
postcode_iso3166_alpha2, postcode_iso3166_alpha2_field, printascii, required,
required_if, required_unless, required_with, required_with_all,
required_without, required_without_all, rgb, rgba, ripemd128, ripemd160,
semver, sha256, sha384, sha512, skip_unless, spicedb, ssn, startsnotwith,
startswith, tcp4, tcp6, tcp, tiger128, tiger160, tiger192,
timezone, udp4, udp6, udp, uds_exists, ulid, unique,
unix_addr, uppercase, uri, url, url_encoded, urn_rfc2141, uuid, uuid3,
uuid3_rfc4122, uuid4, uuid4_rfc4122, uuid5, uuid5_rfc4122, uuid_rfc4122,
validateFn
```

Go 版完整 alias 清单：

```text
country_code, eu_country_code, iscolor
```

### Phase 4 - 嵌套与集合能力

目标：

- 做出接近 Go `dive` 的主要能力，但用 Rust 更自然的写法
- 保留 `dive` 这个词，功能对齐 Go 的 `dive/keys/endkeys`
- Rust 使用 `dive(keys(...), values(...))` 表达 map key/value，`endkeys` 由 `keys(...)` 的右括号天然替代

建议语法：

```rust
#[validate(nested)]
profile: Profile

#[validate(dive(url))]
source_urls: Vec<String>

#[validate(dive(keys(max = 10), values(required)))]
labels: HashMap<String, String>
```

范围：

- 嵌套 struct
- `Vec<T>`
- `HashMap<K, V>`
- `Option<T>`
- 逐项校验

完成标准：

- 能拿到准确的 namespace，如 `source_urls[1]`
- 嵌套错误展开稳定

### Phase 5 - 跨字段与结构体级校验

目标：

- 对齐 Go 里“字段之间关系”和“整体验证”的能力

建议能力：

- `eq_field`
- `ne_field`
- `gt_field`
- `gte_field`
- `lt_field`
- `lte_field`
- `#[validate(check = "fn_name")]`

说明：

- Rust 不必强行复制 Go tag 名
- 不使用 `schema = "..."` 表达 struct-level 校验，避免和 V2 `Schema` 概念混淆
- 重点是表达能力和调用体验
- 简单同级字段比较直接使用 `#[validate(eq_field = "...")]` 这类规则
- 结构体级错误上报使用 `validator::valid::Valid`
- 上报链式 API 使用 `valid.field(...).rule(...).compare(...).param(...).push()`

完成标准：

- 能写密码确认、起止时间比较等经典场景
- struct-level 校验能报告具体字段错误

### Phase 5.5 - 单值校验

目标：

- 对齐 Go `validate.Var(...)`

建议 API：

```rust
Validator::new().value(&email, "required,email")?;
```

完成标准：

- `value(...)` 使用同一套规则与 alias 解析
- 能复用 `Validator::new().alias(...)? .rule(...)?` 链式配置
- 没有 struct 字段时，namespace 语义在实现前单独确认

### Phase 6 - 自定义规则与别名

目标：

- 提供真正可扩展的外部注册能力

建议 API：

```rust
Validator::new()
    .alias("username", "required,length(min=3,max=20)")?
    .rule("slug", Slug)?
    .validate(&user)?;
```

字段使用示意：

```rust
#[validate(alias = "username")]
name: String
```

完成标准：

- 自定义 rule 能接收字段值、namespace、params
- 自定义 rule 的执行上下文命名为 `Field<'_>`，不使用 `RuleInput`
- alias 能展开到统一规则模型

### Phase 7 - 错误模型与 i18n

目标：

- 让错误结果稳定可消费
- 提供 validator 自己的多语言消息渲染能力

建议能力：

- `Error::Failed(Vec<FieldError>)` 支持通过 helper 读取字段错误
- `FieldError` 暴露 `namespace`、`struct_namespace`、`field`、`struct_field`、`kind`、`rule`、`reason`、`params`
- `i18n` 模块提供内置 `zh_cn()` / `en()` Locale，并支持用户自定义 Locale 覆盖

完成标准：

- 不同 rule 返回结构一致
- 后续接中文文案不需要改 executor

### Phase 8 - 性能、缓存与工程打磨

目标：

- 把“能用”推进到“值得长期维护”

范围：

- schema / metadata 缓存
- regex 复用
- 零拷贝或低分配 namespace 构建
- benchmark
- 示例和文档

完成标准：

- 基础 benchmark 建好
- derive 执行链路没有明显的重复分配热点

## V1 与 Go 对齐时的取舍

要对齐的是：

- 使用体验的统一性
- 规则系统的可扩展性
- 嵌套 / 集合 / 跨字段 / struct-level 能力
- 错误结果的可用性

不强行对齐的是：

- Go 的运行时反射风格 API
- Go 的 tag 文本解析细节
- 所有历史兼容行为

Rust 版本应该做成：

- 外部看起来简洁
- 内部比 Go 更分层
- 实现方式更偏编译期元数据 + 运行时执行器

## 当前优先级

P0：

- Phase 0
- Phase 1
- Phase 2

P1：

- Phase 3
- Phase 4
- Phase 5
- Phase 5.5

P2：

- Phase 6
- Phase 7
- Phase 8

V2 已完成：

- `Schema::from_yaml`
- `Schema::from_json`
- `Validator::with_schema`
- `validate_map`

V2 已完成增强：

- `validate_serde`
- derive 选择性校验：`partial` / `except` / `filter`

## 性能缓存

性能缓存作为发布前架构增强项处理，目标是减少重复解析和重复编译，但不改变公开 API。

已实现缓存点：

- direct value 的规则表达式解析和执行编译：`Validator::new().value(&value, "required,email")?` 多次使用同一个表达式时，会复用解析后的 `Vec<Expr>`，并按 validator generation 复用编译后的 `Group`。
- dynamic Schema 的执行树：`Validator::with_schema(schema).validate_map(&data)?` 多次复用同一个 validator / schema 时，会按 `(SchemaId, generation)` 复用编译后的 `Tree`，其中每个字段节点持有对应的执行 `Group`。
- dynamic `regex` 规则：用户传入的 `regex(pattern="...")` 会在同一个 `RegexRule` 实例内缓存编译结果，包括非法 pattern 的失败结果。

性能 benchmark 作为长期维护的工程能力保留，覆盖 derive、direct value
冷/热路径、Schema 冷/热路径、`validate_serde`、nested/dive 和 compound
unique。benchmark 不使用固定耗时阈值作为正确性判断。

设计约束：

- 缓存是内部实现细节，不新增公开 cache 类型。
- 不改 `Validator::new().validate(&value)?`、`value(...)`、`with_schema(...)`、`validate_map(...)` 的公开写法。
- 不改 `Rule`、`Field`、`Value`、`Error`、`FieldError`、`Namespace`、`Params`。
- 不改变 `rule` / `reason` / `Params` 的错误语义。
- 不引入新的校验执行引擎。

和相关功能的关系：

- `validate_serde` 已单独实现；它把 `Serialize` 数据转成 `serde_json::Value` 后复用 `validate_map` 的同一套 Schema `Tree`。
- 不提供兼容别名，避免公开 API 出现双命名。
- i18n 加载已单独实现；`Locale::from_yaml` / `Locale::from_json` 从字符串资源构造 `Locale`，Redis / DB / 配置中心等来源由业务层读取后再传入。
- i18n 不做框架接入抽象；业务层解析 locale 字符串后调用 `i18n.locale(locale).render(fields)`。

## 下一步建议

当前阶段从下面这几个点继续：

1. 做发布前 package/release 收口：检查 crate metadata、README、license、`cargo package --list` 和 derive 包发布顺序。
2. 如果继续补功能，优先从参数化 parser rule 中选择明确业务需求；不要引入兼容层。

已完成：

- `Validator::new().value(&value, "rules")?`
- `Validator::partial(&value, fields)` / `except(...)` / `filter(...)`，共享
  Context 字段选择边界并覆盖 nested、dive 和 struct-level error。
- 显式 `#[validate(nested)]`
- `dive(...)` for `Vec` / array / slice
- `dive(keys(...), values(...))` for map
- `validator::valid::Valid`
- `#[validate(check = "...")]`
- `eq_field` / `ne_field` / `gt_field` / `gte_field` / `lt_field` / `lte_field` derive 跨字段规则，支持同级字段和相对向下点分目标
- Schema 中等价的 `*_field` 同级/嵌套对象字段规则，并在 Tree 编译时校验路径
- Rust 原生 `std::time::SystemTime` 时间点校验
- 内部 field access layer：derive 生成按路径选择的借用链，Schema Scope 缓存实际引用路径；未来反射后端只替换这一层
- `nested` / `dive` / `value(...)` 的 namespace 规则设计
- `Schema::from_yaml`
- `Schema::from_json`
- `Validator::with_schema(schema).validate_map(&data)`
- `Validator::with_schema(schema).validate_serde(&value)`
- `examples/` 下按使用场景拆分的独立示例
- 公开规则参数统一为结构化 `Param` / `Params`，通过 `text(...)`、`list(...)`、`pairs(...)` 读取；parser 内部未绑定参数使用 `RawParam` / `RawParams`。
- Rule 与 Alias 使用单一 Registry 名称空间，重复注册返回 `Error::DuplicateName`，Alias 循环返回 `Error::RecursiveAlias`。
- `Rule::signature()` 统一约束 derive、单值和 Schema 参数绑定，字段规则通过 `Field::sibling(...)` 进入同一执行路径。
- 单目标自定义跨字段规则使用 `*_field` 命名和 `Signature::field("compare")`；derive 据此生成目标字段访问，复杂多字段逻辑使用 struct-level `check`。
- runtime expression 严格拒绝空规则、空 alternative、不平衡括号和未闭合引号；alias 不能为空，多层 alias 保留最外层 `rule`。
- Schema type 仅接受 `string`、`boolean`、`int`、`uint`、`float`、`array`、`object`；Locale 资源仅接受 `locale`。
- `validator::i18n` message rendering，包含内置 `zh_cn()` / `en()`、用户 `Locale` 覆盖、locale fallback、模板查找和 `Template::Fn`。
- `Locale::from_yaml` / `Locale::from_json`，用于从字符串资源加载用户自定义 locale。
- i18n 覆盖率 guard，防止新增内置 rule / alias 后遗漏 `zh-CN` / `en` 默认文案。
- V1 network/equality common rules: `eq`, `ne`, `http`, `https`, `ip`, `ipv4`, `ipv6`, `uuid`。
- V1 string/choice common rules: `ascii`, `containsany`, `noneof`，并且 `oneof` / `noneof` 支持字符串、有符号整数和无符号整数的类型分派。
- V1 format/network common rules: `uri`, `cidr`, `cidrv4`, `cidrv6`, `hostname`, `hostname_rfc1123`, `fqdn`, `port`, `uuid3`, `uuid4`, `uuid5`, `json`, `datetime`。
- V1 identifier rule: `ulid`。
- V1 collection rule: `unique`，支持无参数数组/切片/Vec 去重、Map values 去重，以及 Vec/数组/切片结构体元素的单字段、复合字段和嵌套路径投影；Schema array fields 使用相同字段投影语义。
- alias 内 `omitempty` 会跨 Group/derive 边界停止当前字段剩余规则。
- Schema / Locale 严格拒绝未知键，Choice Schema 简写统一使用 `values` 参数。
- URL / URI 使用结构化解析，`hostname` / `hostname_rfc1123` / `fqdn` 保留各自语义。
- 条件 required/excluded、`fieldcontains`、`fieldexcludes`、`isdefault`、`hostname_port` 已实现并有测试与 i18n 覆盖。
- README / README.zh-CN / examples 已展示最新 format/network/identifier 规则的单值和结构体校验用法。
