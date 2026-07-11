# 架构说明

[English](architecture.md)

## 设计说明：反射

项目目前处于初始开发阶段，不保留旧 API 或旧配置写法。类型名、规则名或 API 边界设计错误时，直接替换实现，不增加兼容 alias、deprecated wrapper 或回退解析。出现需要兼容补丁的提议时，优先重新检查底层架构，而不是把兼容分支叠加到现有设计上。

Go 版 validator 可以直接依赖语言级运行时反射来读取结构体字段、字段类型和值。Rust 当前没有等价的内置结构体反射能力。生态里的反射库通常也要求用户额外 `derive` 一个反射 trait，库才能在运行时读取字段信息。

因此，`validator` 当前把用户 API 收敛在 `#[derive(Validate)]` 上，由 derive 宏生成校验引擎需要的轻量字段元数据和访问代码。这样用户不需要再额外写一个反射 derive，同时规则执行、`Value` 类型分派、错误结果和 i18n 仍然保持在同一套核心模型里。
生成的访问代码是按需的：只有校验属性实际引用的直接字段和完整嵌套目标才会进入访问层。

这一层是内部实现细节。后续如果 Rust 本身提供成熟反射能力，或者某个反射库可以足够干净地隐藏在 `validator` 内部，我们可以把字段访问层替换成反射实现，而不改变外部的校验 DSL。

替换边界必须保持很窄：未来的 Rust 反射或 `facet` 后端只能替换 validator 如何发现字段、读取字段值，不能替换公开的 `#[validate(...)]` DSL、规则注册表、`Value` / `Kind` 语义、`Error` / `FieldError` 错误模型、Schema 规则语义和 i18n 渲染。也就是说，反射只是一种字段访问后端，不是另一套校验引擎。


## 当前边界

这些是当前 API 有意保留的边界：

- 参数化 `unique` 支持 Vec、数组、切片引用和 Schema object arrays 的单字段、复合字段及嵌套元素路径；不支持 Map、原生 direct value，也不能写在 `dive(...)` 内。
- 嵌套字段目标只支持相对当前结构体向下解析；不支持父级/root、集合索引、Map key，也不支持条件规则路径。
- derive 代码必须显式写出元素字段路径，运行时 alias 无法生成 Rust 字段访问；Schema 在运行时已有字段定义，因此 Schema alias 可以包含参数化 unique 表达式。
- 原生集合使用 `dive(...)` 之外的通用规则时，元素或 Map value 类型必须实现 `Value`；纯 `dive(nested)` 只要求元素实现 `Validate`。
- 原生时间校验只支持 `std::time::SystemTime`；不内置 `Duration`、`chrono`、`time` crate 值。
- 暂不内置 `country_code` 以及相关国家代码 alias。
- 不内置 Web/RPC 框架集成；业务或外部 adapter 决定 locale 和响应格式。
- 不要求用户使用 Rust 运行时反射。字段访问由 `#[derive(Validate)]` 生成，并保持在公开校验 DSL 背后。
