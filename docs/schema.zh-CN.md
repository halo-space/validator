# Schema 校验

[English](schema.md)

## 动态 Schema 校验

当规则来自 YAML / JSON 配置，而不是 Rust 字段属性时，可以使用 `Schema`。

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
这条路径复用同一套规则注册表、alias、`Value` 类型分派、`Error` 和 `Namespace` 模型，不是另一套校验引擎。

Schema 的单目标字段规则使用同一套相对点分语法：

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

每个中间 Schema 段都必须声明为 `object`；这项能力不会穿过 array。运行时中间段缺失或为 null 时，目标按缺失处理，同时保留终点字段声明的 `Kind`。Schema alias 可以包含点分字段规则，因为 alias 展开和路径校验都发生在 Schema Tree 编译阶段。单段目标按序列化字段名精确匹配，因此 `source-url` 这类名称仍然有效；包含 `.` 的目标固定表示嵌套路径。如果同一层还声明了同名字面点分字段，Schema 编译会直接拒绝歧义配置，而不是按查找优先级选择其中一个。

Schema 配置采用严格模式：顶层只允许 `fields`，字段定义只允许 `type`、`rules` 和 `fields`。`type` 只接受 `string`、`boolean`、`int`、`uint`、`float`、`array`、`object`，只有 `object` 和 `array` 可以定义嵌套 `fields`。即使 `fields: {}` 为空，它仍然表示结构声明：省略 `type` 时推断为 `object`，标量类型配置空 `fields` 仍会报错，array 配置空 `fields` 时每个元素仍必须是 object；完全没有 `fields` 的 array 不限制元素必须是 object。未知键或未知类型返回 `Error::InvalidSchema`；不符合 Rule `Signature` 的参数在 Schema 编译时返回 `Error::InvalidRuleExpression`。语义参数也会在缓存 Schema Tree 构建时完成预检，早于根数据或字段数据的类型检查短路。

Schema 面向 JSON/YAML 数据。它支持把 `datetime` 当成字符串规则使用，但不支持原生 `SystemTime` 值，也不支持 `type: time`。数字规则按 Schema 声明族执行：`int` 使用有符号整数，`uint` 使用无符号整数，`float` 只接受浮点形式。整数值不能通过 `type: float`，浮点值也不能通过 `type: int`。

Schema 类型名与校验规则名属于不同命名空间。`type: float` 声明字段的数据类型；`number` 是校验谓词，不是类型名。`number` 接受原生数值，以及只包含 ASCII 数字的字符串，例如 `"12345"`，但不接受正负号和小数点。`numeric` 同样接受原生数值，它的字符串形式还可以包含前导正负号和小数部分，例如 `"-12.5"`。因此 `number` 可以写在 `rules` 中，但不能写成 Schema `type`。

`type: array` 下的 `fields` 描述对象元素，可以直接按元素字段去重：

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

元素字段错误使用 `users[0].email` namespace；包括 `null` 在内的非对象元素使用 `users[0]` namespace 返回 `type` 错误。`unique: email` 是单字段列表简写。如果单字段或复合字段投影到的输入不符合子字段类型，unique 会跳过该元素的完整键，由子字段返回带索引的 `type` 错误；错误输入不会被误判成规则配置错误。

如果数据已经实现了 `serde::Serialize`，可以直接使用 `validate_serde(...)`。
Schema 字段名跟随序列化后的数据结构，包括 `serde(rename)`、`serde(rename_all)`、`serde(skip_serializing_if)` 和 `serde(flatten)`。
`validate_serde` 会先取得并编译 Schema，再调用用户的序列化实现，因此 `MissingSchema` 或非法 Schema 配置不会被自定义 serializer 错误遮住。

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
