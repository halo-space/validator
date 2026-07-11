# I18n Design Notes

本文记录当前已经讨论确认并实现的 i18n 设计。

## 定位

i18n 是 validator 的错误消息渲染层：输入 `FieldError` 和 locale，输出本地化后的 `Message` 列表。

i18n 不负责执行校验，不改变 `Error::Failed(Vec<FieldError>)` 的结构，也不从 HTTP header、RPC metadata、用户配置里获取语言。语言来源由业务代码决定，然后传给 i18n。

典型场景：

- Web API 根据 `Accept-Language` 返回中文或英文错误。
- RPC 服务把结构化校验错误转成客户端可展示文案。
- CLI 或后台任务固定输出某一种语言。
- 多租户或业务系统覆盖默认规则文案和字段显示名。

## 核心概念

- `Locale`: 一种语言或地区的一套翻译资源，例如 `zh-CN`、`en`。
- `Catalog`: 多个 `Locale` 的集合，负责 fallback 和渲染入口。
- `Message`: 一条渲染后的错误消息，保留结构化错误上下文和最终文案。
- `Translator`: 通过 `Catalog::locale(...)` 选择本次渲染语言后得到的本次语言翻译器。
- `Template`: 某条 rule 的文案生成方式，可以是字符串模板，也可以是动态函数。
- `Context`: `rule_fn` 执行时拿到的上下文。

`Locale` 负责资源，`Catalog` 负责组织资源，`Translator` 负责把 `FieldError` 渲染成 `Message`。

## 主路径 API

动态语言是主路径：

```rust
let zh = validator::i18n::Locale::new("zh-CN")
    .rule("email", "{field}格式不正确")
    .field("email", "邮箱");

let i18n = validator::i18n::new()
    .zh_cn()
    .en()
    .use_locale(zh)
    .fallback("zh-CN");

let messages = i18n
    .locale(locale_from_header)
    .render(error.fields().unwrap());
```

也支持一次性链式写法：

```rust
let messages = validator::i18n::new()
    .zh_cn()
    .en()
    .use_locale(zh)
    .fallback("zh-CN")
    .locale(locale_from_header)
    .render(error.fields().unwrap());
```

固定语言快捷方式：

```rust
let messages = validator::i18n::zh_cn()
    .render(error.fields().unwrap());
```

快捷方式适合固定中文项目、CLI、测试；动态业务主路径仍然是 `i18n.locale(locale).render(fields)`。

## Locale 资源

`Locale` 表示某一种语言下的规则文案和字段显示名。

Rust 构造示例：

```rust
let zh = validator::i18n::Locale::new("zh-CN")
    .rule("required", "{field}不能为空")
    .rule("email", "{field}格式不正确")
    .rule("min", "{field}不能小于{min}")
    .rule("max", "{field}不能大于{max}")
    .field("email", "邮箱")
    .field("name", "用户名")
    .field("age", "年龄");
```

`.rule(...)` 注册的是字符串模板，适合用户从 YAML、JSON、DB、Redis、配置中心等外部来源覆盖文案。

复杂规则可以使用 `.rule_fn(...)` 注册动态函数。动态函数适合 `min`、`max`、`length`、`gt`、`gte`、`lt`、`lte`、`range` 这类需要根据字段类型、参数或复数规则选择不同文案的场景。

```rust
let zh = validator::i18n::Locale::new("zh-CN")
    .rule("email", "{field}格式不正确")
    .rule_fn("min", |ctx| {
        let min = ctx.param("min").unwrap_or_default();
        match ctx.kind() {
            validator::Kind::String => format!("{}长度不能小于{}", ctx.field(), min),
            _ => format!("{}不能小于{}", ctx.field(), min),
        }
    })
    .field("email", "邮箱");
```

`Context` 至少需要暴露：

```rust
impl<'a> Context<'a> {
    pub fn namespace(&self) -> &Namespace;
    pub fn field(&self) -> &str;
    pub fn rule(&self) -> &str;
    pub fn reason(&self) -> &str;
    pub fn kind(&self) -> Kind;
    pub fn params(&self) -> &Params;
    pub fn param(&self, name: &str) -> Option<&str>;
    pub fn param_list(&self, name: &str) -> Option<&[String]>;
    pub fn param_pairs(&self, name: &str) -> Option<&[(String, String)]>;
}
```

`kind()` 来自校验失败时记录的字段语义类型，用于内置 `Template::Fn` 区分字符串长度、集合长度、数字大小等文案。

也可以直接注册 `Template`，用于高级场景：

```rust
let zh = validator::i18n::Locale::new("zh-CN")
    .template("email", validator::i18n::Template::Text("{field}格式不正确".to_owned()));
```

对应数据含义：

```text
Locale {
    locale: "zh-CN",
    rules: {
        "required": "{field}不能为空",
        "email": "{field}格式不正确",
        "min": "{field}不能小于{min}",
        "max": "{field}不能大于{max}",
    },
    fields: {
        "email": "邮箱",
        "name": "用户名",
        "age": "年龄",
    },
}
```

内部资源结构可以按这个模型设计：

```rust
pub struct Locale {
    locale: String,
    rules: BTreeMap<String, Template>,
    fields: BTreeMap<String, String>,
}

pub enum Template {
    Text(String),
    Fn(RenderFn),
}
```

`Template::Text` 是普通模板，`Template::Fn` 是动态文案函数。

`RenderFn` 支持用户闭包和共享状态，当前实现采用这个形态：

```rust
pub type RenderFn = Arc<dyn for<'a> Fn(&Context<'a>) -> String + Send + Sync + 'static>;
```

YAML 资源使用这个结构：

```yaml
locale: zh-CN

rules:
  required: "{field}不能为空"
  email: "{field}格式不正确"
  min: "{field}不能小于{min}"
  max: "{field}不能大于{max}"

fields:
  email: "邮箱"
  name: "用户名"
  age: "年龄"
```

JSON 资源使用这个结构：

```json
{
  "locale": "zh-CN",
  "rules": {
    "required": "{field}不能为空",
    "email": "{field}格式不正确",
    "min": "{field}不能小于{min}",
    "max": "{field}不能大于{max}"
  },
  "fields": {
    "email": "邮箱",
    "name": "用户名",
    "age": "年龄"
  }
}
```

Redis、DB、配置中心等来源由用户自己读取，再转换成 `Locale`。主 API 提供 `Locale::from_yaml` / `Locale::from_json` 用来从字符串资源构造 `Locale`；不直接提供 Redis、DB、配置中心 SDK 集成。

```rust
let yaml = redis.get::<_, String>("validator:i18n:zh-CN")?;
let zh = validator::i18n::Locale::from_yaml(&yaml)?;

let i18n = validator::i18n::new()
    .zh_cn()
    .use_locale(zh)
    .fallback("zh-CN");
```

validator 不直接依赖 Redis client、数据库驱动或配置中心 SDK。

Locale 资源只接受 `locale` 作为语言标识键，不接受 `name` 兼容写法。代码中通过 `Locale::locale()` 读取该标识：

```rust
assert_eq!(zh.locale(), "zh-CN");
```

## 合并规则

`use_locale(locale)` 表示把一个已经构造好的 `Locale` 纳入当前 `Catalog` 配置，并与已有同名 locale 合并。

合并发生在初始化阶段，不在 render 时做“先找用户自定义，再找内置”的双层查找。

合并规则：

```text
先加载内置 locale
再 use_locale 用户 locale
同 locale 下用户 rule / field 覆盖内置 rule / field
render 时只查合并后的最终表
```

示例：

```rust
let zh = validator::i18n::Locale::new("zh-CN")
    .rule("email", "请输入正确的{field}")
    .field("email", "邮箱");

let i18n = validator::i18n::new()
    .zh_cn()
    .use_locale(zh)
    .fallback("zh-CN");
```

这里用户提供的 `email` 文案和字段名会覆盖内置 `zh-CN` 的对应配置。

## locale 选择语义

`.locale("zh-CN")` 不是修改 `Catalog` 本身，而是选择本次渲染要使用的语言视图。

```rust
let zh = i18n.locale("zh-CN");
let en = i18n.locale("en");

let zh_messages = zh.render(fields);
let en_messages = en.render(fields);
```

`Catalog` 本身保持不变，适合在 Web/RPC 服务里全局共享。`locale(...)` 返回的 `Translator` 是本次请求的轻量语言选择。

如果传入 locale 不存在，则走 `fallback(...)` 配置的 locale。fallback locale 也不存在时，不返回新的配置错误，而是使用内置默认错误文本渲染 `Message`。

默认错误文本只作为最后兜底，保证 i18n 不影响校验错误本身的可用性。当前实现固定为：`{namespace} failed {rule}`。

## Message 结构

`Message` 是 `FieldError` 的翻译视图，不是简单字符串。它保留结构化上下文，同时带上最终文案。

```rust
pub struct Message {
    pub namespace: Namespace,
    pub struct_namespace: Namespace,
    pub field: String,
    pub struct_field: String,
    pub rule: String,
    pub reason: String,
    pub kind: Kind,
    pub params: Params,
    pub text: String,
}
```

字段含义：

- `namespace`: 对外字段路径，例如 `User.email`、`items[0].name`。
- `struct_namespace`: Rust struct 原始字段路径。
- `field`: 对外字段名。
- `struct_field`: Rust struct 原始字段名。
- `rule`: 用户声明的规则名；alias 场景下是 alias 名。
- `reason`: 真实失败原因；普通规则下等于 `rule`，alias 场景下是底层失败规则。
- `kind`: 字段语义类型，例如 `Kind::String`、`Kind::Int(_)`、`Kind::Map`。
- `params`: 规则参数，例如 `{ min: 3, max: 20 }`。
- `text`: 翻译后的最终文案。

普通规则例子：

```text
rule = "email"
reason = "email"
text = "邮箱格式不正确"
```

alias 例子：

```text
rule = "username"
reason = "length"
params = { "min": "3", "max": "20" }
text = "用户名长度必须在3到20之间"
```

颜色 alias 例子：

```text
rule = "iscolor"
reason = "hexcolor|rgb|rgba|hsl|hsla|cmyk"
```

## rule 与 reason

`rule` 是用户声明的规则名，或者 alias 名。

`reason` 是真正失败的底层规则。

例如：

```rust
let validator = Validator::new()
    .alias("username", "required,length(min=3,max=20)")?;

#[derive(Validate)]
struct Account {
    #[validate(alias = "username")]
    name: String,
}
```

如果 `name = "ab"`，失败原因是 `length`：

```text
rule = "username"
reason = "length"
params = { "min": "3", "max": "20" }
```

如果 `name = ""`，失败原因是 `required`：

```text
rule = "username"
reason = "required"
params = {}
```

i18n 查模板顺序：

```text
rule template -> reason template -> default template/text
```

也就是说，业务如果给 `username` 配了专用文案，就用 `username`；没有专用文案时，再退回到底层 `length` / `required` 的通用文案。

这里的 `default template/text` 是模板兜底，不是 `.fallback("zh-CN")` 的 locale fallback。locale fallback 只负责选择语言资源，模板兜底只负责当前语言资源里找不到对应 rule/reason 文案时生成最终文本。

查到模板后的渲染方式：

```text
Template::Text -> 做模板替换
Template::Fn   -> 调用 rule_fn 动态生成 text
```

`Template::Text` 只扫描原始模板一次。字段显示名和规则参数替换进去后按普通文本输出，即使值本身包含 `{rule}`、`{field}` 这类内容，也不会再次解释为占位符；原始模板中的未知占位符保持不变。

内置 `zh_cn()` / `en()` 可以使用 `Template::Fn` 处理复杂规则；用户自定义 `Locale` 可以用 `.rule(...)` 覆盖内置模板或内置函数。

合并时后注册的同名 rule 覆盖先注册的同名 rule，因此用户模板可以覆盖内置动态函数：

```rust
let i18n = validator::i18n::new()
    .zh_cn()
    .use_locale(
        validator::i18n::Locale::new("zh-CN")
            .rule("min", "{field}太小了")
    );
```

这里用户的 `min` 字符串模板会覆盖内置 `min` 动态函数。

## Go 对齐点

Go validator 的翻译链路是：

```go
trans, _ := uni.GetTranslator("en")
en_translations.RegisterDefaultTranslations(validate, trans)
messages := errs.Translate(trans)
```

Rust 版借鉴它的核心思想：外部决定 locale，再把 locale 传给翻译层。但 Rust 版不照搬 `universal-translator` 和注册函数复杂度，而是用 `Catalog + Locale + Translator` 直接表达。

Go validator 的 i18n 不是纯模板替换。它内部保存 `translator -> tag -> TranslationFunc`，普通规则使用模板和通用翻译函数，复杂规则使用自定义翻译函数按字段类型和参数选择不同模板。

Rust 版对应设计为：

```text
用户自定义 Locale: 主要提供 Template::Text 覆盖
内置 Locale: 可以提供 Template::Text + Template::Fn
高级扩展: 通过 rule_fn / template 开放自定义动态函数
```

Go 的批量翻译返回 `map[namespace]string`，Rust 版不照搬。因为同一个字段可能有多条错误，map 会覆盖，所以 Rust 版使用 `Vec<Message>`。

Go validator 和 Gin 的绑定边界也需要明确：

- Go validator 主库只提供 `RegisterTranslation(...)`、`FieldError.Translate(trans)`、`ValidationErrors.Translate(trans)` 这类翻译入口。
- Gin 示例只是实现 Gin 自己的 `binding.StructValidator`，内部调用 `validate.Struct(obj)`，不属于 validator 主库的 i18n 设计。
- HTTP header、RPC metadata、CLI 参数、租户配置等 locale 来源都属于业务层。业务层解析出 `zh-CN` / `en` 后传给 `i18n.locale(locale)`。

因此 Rust 版不新增框架接入抽象。`i18n.locale(locale).render(fields)` 就是稳定边界。

## 非目标

- 不内置 Web/RPC 框架适配。
- 不从 HTTP header 或 RPC metadata 中读取 locale。
- 不提供框架接入抽象；业务或外部 adapter 直接解析 locale 字符串并调用 `i18n.locale(locale).render(fields)`。
- 不依赖 Redis、DB、配置中心 SDK。
- 不改变校验执行模型。
- 不把 `FieldError` 变成字符串后丢掉结构化信息。
