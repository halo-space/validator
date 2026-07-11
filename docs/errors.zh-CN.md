# 错误模型

[English](errors.md)

## 错误结果

所有公开校验入口都返回 `Error`。校验失败使用 `Error::Failed(Vec<FieldError>)`；未知规则、重复名称、错误参数、缺少字段上下文、递归 Alias、Schema 或数据转换错误使用对应的配置错误变体。参数配置预检始终先于数据相关短路，因此字段缺失、为空或被跳过时，错误参数仍会直接返回。

| 变体 | 含义 |
| --- | --- |
| `Failed` | 一个或多个字段校验失败。 |
| `UnknownRule` | 运行时表达式或 alias 引用了未注册规则。 |
| `UnknownField` | selector 或结构体级错误引用了未声明的根字段。 |
| `InvalidRuleExpression` | 规则语法、参数形状或参数语义错误。 |
| `InvalidRuleName`, `InvalidAliasName` | 注册名称包含不支持的字符。 |
| `DuplicateName` | rule、alias 或保留控制名称已经注册。 |
| `MissingFieldContext` | 单值入口使用了依赖字段上下文的规则。 |
| `RecursiveAlias` | alias 展开出现循环。 |
| `MissingSchema` | 未配置 Schema 就调用了 Schema 校验入口。 |
| `InvalidSchema` | Schema 资源格式错误或内部配置冲突。 |
| `InvalidData` | 序列化或 locale 资源解析失败。 |

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

每个 `FieldError` 暴露：

- `namespace`
- `struct_namespace`
- `field`
- `struct_field`
- `kind`
- `rule`
- `reason`
- `params`
