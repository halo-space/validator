# Rule Reference

[中文](rules.zh-CN.md)

## Built-In Rules

Current built-in rules:

- Required/Optional: `required`, `isdefault`, `omitempty`
- Size: `length`, `min`, `max`, `range`
- Compare: `eq`, `ne`, `eq_ignore_case`, `ne_ignore_case`, `gt`, `gte`,
  `lt`, `lte`
- Field-aware: `eq_field`, `ne_field`, `gt_field`, `gte_field`, `lt_field`,
  `lte_field`, `fieldcontains`, `fieldexcludes`, `required_if`,
  `required_unless`, `skip_unless`, `required_with`, `required_with_all`,
  `required_without`, `required_without_all`, `excluded_if`, `excluded_unless`,
  `excluded_with`, `excluded_with_all`, `excluded_without`,
  `excluded_without_all` for derive and Schema validation
- Collection: `unique`
- Choice: `oneof`, `oneofci`, `noneof`, `noneofci`
- String: `contains`, `containsany`, `containsrune`, `excludes`,
  `excludesall`, `excludesrune`, `startswith`, `endswith`, `startsnotwith`,
  `endsnotwith`, `ascii`, `printascii`, `multibyte`, `alpha`, `alphaspace`,
  `alphaunicode`, `alphanum`, `alphanumspace`, `alphanumunicode`, `numeric`,
  `number`, `lowercase`, `uppercase`, `boolean`

  Here `number` means an ASCII-digit predicate for strings, not a Schema type;
  `numeric` additionally accepts a sign and decimal fraction.
- Format: `email`, `regex`, `json`, `datetime`, `e164`, `base32`, `base64`,
  `base64url`, `base64rawurl`, `hexadecimal`, `url_encoded`, `html`,
  `html_encoded`, `jwt`, `mac`, `semver`, `origin`, `datauri`, `latitude`,
  `longitude`, `ssn`, `md4`, `md5`, `sha256`, `sha384`, `sha512`,
  `ripemd128`, `ripemd160`, `tiger128`, `tiger160`, `tiger192`, `eth_addr`,
  `mongodb`, `mongodb_connection_string`, `dns_rfc1035_label`, `cve`, `cron`,
  `ein`, `bic_iso_9362_2014`, `bic`, `isbn`, `isbn10`, `isbn13`, `issn`,
  `credit_card`, `luhn_checksum`, `hexcolor`, `rgb`, `rgba`, `hsl`, `hsla`,
  `cmyk`
- Network: `url`, `uri`, `http`, `https`, `ip`, `ipv4`, `ipv6`,
  `cidr`, `cidrv4`, `cidrv6`, `hostname`,
  `hostname_port`, `hostname_rfc1123`, `fqdn`, `port`, `uuid`, `uuid3`,
  `uuid4`, `uuid5`, `uuid_rfc4122`, `uuid3_rfc4122`, `uuid4_rfc4122`,
  `uuid5_rfc4122`, `ulid`, `tcp4`, `tcp6`, `tcp`, `udp4`, `udp6`, `udp`
- Alias: `iscolor`

## Parameter Signatures

Rule parameters are strict. Positional and named parameters cannot be mixed in
one rule, unknown names are rejected, and semantic validation happens before
data-dependent short-circuiting.

| Rules | Parameters | Supported values |
| --- | --- | --- |
| `required`, `isdefault`, `omitempty` | none | any `Value` |
| `length` | named `exact`, `min`, `max` | strings and collections |
| `min`, `max` | one text value | strings, collections, `int`, `uint`, `float` |
| `range` | required named `min`, `max` | strings, collections, `int`, `uint`, `float` |
| `eq`, `ne`, `gt`, `gte`, `lt`, `lte` | optional `value`; ordered time rules may omit it | strings, collections, numeric values, and supported time comparisons |
| `eq_ignore_case`, `ne_ignore_case` | required `value` | strings |
| `*_field`, `fieldcontains`, `fieldexcludes` | one target field named `compare` | derive and Schema field contexts |
| `required_if`, `required_unless`, `excluded_if`, `excluded_unless` | one or more `field=value` pairs | derive and Schema field contexts |
| `required_with*`, `required_without*`, `excluded_with*`, `excluded_without*`, `skip_unless` | one or more field names | derive and Schema field contexts |
| `unique` | optional field list `fields` | collections; parameterized form requires element field access |
| `oneof`, `oneofci`, `noneof`, `noneofci` | one or more `values` | strings, `int`, and `uint` |
| `contains*`, `excludes*`, `startswith`, `endswith`, `startsnotwith`, `endsnotwith` | required `value` | strings |
| `regex` | required `pattern` | strings |

Equivalent expression forms include:

```text
min=3
range(min=1,max=10)
oneof("draft","published")
required_if(status="draft")
unique(email,profile.id)
```

Rules not listed in the table take no parameters. Their accepted kinds follow
the family descriptions below; unsupported kinds fail validation or return an
`InvalidRuleExpression` when the rule configuration itself is impossible.

Ordered comparison and size rules dispatch by field type:

- Strings use character count.
- Vectors, arrays, slices, and maps use item count.
- `int`, `uint`, and `float` values use their own numeric families.
- `std::time::SystemTime` supports no-parameter time comparison against a
  captured `now` and same-kind `*_field` comparison.
- `Option::None` skips non-`required` rules and fails `required`.

Equality rules compare string content instead of length. `length` rejects an
empty configuration and does not allow `exact` together with `min` or `max`;
`length` and `range` reject reversed bounds during parameter preflight.

Choice rules dispatch by field type for strings, `int` values, and `uint` values.
URL and URI rules use structured parsers. `hostname` follows RFC952, while
`hostname_rfc1123` permits a leading digit and `fqdn` requires a non-numeric TLD.
`cidr` accepts IPv4 or IPv6 address-prefix notation, while `cidrv4` additionally
requires a canonical network address. `mac` accepts 6-, 8-, and 20-octet link
addresses, and lowercase `uuid4` / `uuid5` check both version and RFC variant.
