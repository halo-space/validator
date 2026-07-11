# 规则参考

[English](rules.md)

## 内置规则

当前内置规则：

- 必填/可选: `required`, `isdefault`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `eq`, `ne`, `eq_ignore_case`, `ne_ignore_case`, `gt`, `gte`, `lt`, `lte`
- Field-aware: `eq_field`, `ne_field`, `gt_field`, `gte_field`, `lt_field`, `lte_field`, `fieldcontains`, `fieldexcludes`, `required_if`, `required_unless`, `skip_unless`, `required_with`, `required_with_all`, `required_without`, `required_without_all`, `excluded_if`, `excluded_unless`, `excluded_with`, `excluded_with_all`, `excluded_without`, `excluded_without_all`，用于 derive 和 Schema 校验
- Collection: `unique`
- Choice: `oneof`, `oneofci`, `noneof`, `noneofci`
- String: `contains`, `containsany`, `containsrune`, `excludes`, `excludesall`, `excludesrune`, `startswith`, `endswith`, `startsnotwith`, `endsnotwith`, `ascii`, `printascii`, `multibyte`, `alpha`, `alphaspace`, `alphaunicode`, `alphanum`, `alphanumspace`, `alphanumunicode`, `numeric`, `number`, `lowercase`, `uppercase`, `boolean`

  这里的 `number` 表示字符串的 ASCII 数字谓词，不是 Schema 类型；`numeric` 还允许正负号和小数部分。
- Format: `email`, `regex`, `json`, `datetime`, `e164`, `base32`, `base64`, `base64url`, `base64rawurl`, `hexadecimal`, `url_encoded`, `html`, `html_encoded`, `jwt`, `mac`, `semver`, `origin`, `datauri`, `latitude`, `longitude`, `ssn`, `md4`, `md5`, `sha256`, `sha384`, `sha512`, `ripemd128`, `ripemd160`, `tiger128`, `tiger160`, `tiger192`, `eth_addr`, `mongodb`, `mongodb_connection_string`, `dns_rfc1035_label`, `cve`, `cron`, `ein`, `bic_iso_9362_2014`, `bic`, `isbn`, `isbn10`, `isbn13`, `issn`, `credit_card`, `luhn_checksum`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`, `cmyk`
- Network: `url`, `uri`, `http`, `https`, `ip`, `ipv4`, `ipv6`, `cidr`, `cidrv4`, `cidrv6`, `hostname`, `hostname_port`, `hostname_rfc1123`, `fqdn`, `port`, `uuid`, `uuid3`, `uuid4`, `uuid5`, `uuid_rfc4122`, `uuid3_rfc4122`, `uuid4_rfc4122`, `uuid5_rfc4122`, `ulid`, `tcp4`, `tcp6`, `tcp`, `udp4`, `udp6`, `udp`
- Alias: `iscolor`

## 参数签名

规则参数采用严格模式：同一条规则不能混用位置参数和命名参数，未知参数名会被拒绝，
参数语义校验早于数据相关的短路逻辑。

| 规则 | 参数 | 支持的数据 |
| --- | --- | --- |
| `required`, `isdefault`, `omitempty` | 无参数 | 任意 `Value` |
| `length` | 命名参数 `exact`, `min`, `max` | 字符串和集合 |
| `min`, `max` | 一个文本参数 | 字符串、集合、`int`、`uint`、`float` |
| `range` | 必填命名参数 `min`, `max` | 字符串、集合、`int`、`uint`、`float` |
| `eq`, `ne`, `gt`, `gte`, `lt`, `lte` | 可选 `value`；有序时间规则可以省略 | 字符串、集合、数值及受支持的时间比较 |
| `eq_ignore_case`, `ne_ignore_case` | 必填 `value` | 字符串 |
| `*_field`, `fieldcontains`, `fieldexcludes` | 一个名为 `compare` 的目标字段 | derive 和 Schema 字段上下文 |
| `required_if`, `required_unless`, `excluded_if`, `excluded_unless` | 一个或多个 `field=value` | derive 和 Schema 字段上下文 |
| `required_with*`, `required_without*`, `excluded_with*`, `excluded_without*`, `skip_unless` | 一个或多个字段名 | derive 和 Schema 字段上下文 |
| `unique` | 可选字段列表 `fields` | 集合；参数化形式要求元素字段访问 |
| `oneof`, `oneofci`, `noneof`, `noneofci` | 一个或多个 `values` | 字符串、`int`、`uint` |
| `contains*`, `excludes*`, `startswith`, `endswith`, `startsnotwith`, `endsnotwith` | 必填 `value` | 字符串 |
| `regex` | 必填 `pattern` | 字符串 |

等价的表达式写法包括：

```text
min=3
range(min=1,max=10)
oneof("draft","published")
required_if(status="draft")
unique(email,profile.id)
```

表中未列出的规则不接受参数。它们支持的数据类型由下面的规则族说明决定；不支持的
类型会校验失败，配置本身不可能成立时则返回 `InvalidRuleExpression`。

有序比较和尺寸类规则会根据字段类型分派：

- 字符串按字符数量比较。
- `Vec`、数组、切片、Map 按元素数量比较。
- 有符号整数、无符号整数、浮点数分别按自己的数值族处理。
- `std::time::SystemTime` 支持无参数时间比较，也支持同类型 `*_field` 字段间比较。
- `Option::None` 会跳过非 `required` 规则，但会让 `required` 失败。

Equality 规则比较字符串内容，不比较长度。`length` 不允许空参数，也不允许 `exact` 与 `min` / `max` 混用；`length` 和 `range` 会在参数预检阶段拒绝倒置边界。

Choice 类规则会根据字段类型分派，当前支持字符串、有符号整数和无符号整数。
URL 和 URI 使用结构化解析器；`hostname` 遵循 RFC952，`hostname_rfc1123` 允许数字开头，`fqdn` 要求非数字顶级域名。
`cidr` 接受 IPv4 或 IPv6 address-prefix，`cidrv4` 额外要求 canonical network address。`mac` 支持 6、8、20 octet 链路层地址，`uuid4` / `uuid5` 的小写规则同时检查 version 和 RFC variant。
