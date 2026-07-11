# Default Rules Coverage

This document tracks coverage against the Go validator default rule registry in
[`go-playground/validator`](https://github.com/go-playground/validator/blob/master/baked_in.go).

This version is audited against the actual Go implementations, not just rule
names. The key lesson is that names can be misleading: some `*_addr` rules are
local address parsers, while `image` and `mimetype` read files from disk.

Status labels:

- `Done`: implemented in Rust `validator` with tests and i18n coverage.
- `Next`: can be implemented as pure validation logic without changing the
  execution model.
- `Design`: useful, but needs an API or execution-model decision first.
- `Later`: technically possible, but needs a dependency decision, system data,
  filesystem access, or larger maintenance commitment.
- `Won't Do`: intentionally excluded from the core crate.

## Naming Notes

- Go `len` maps to Rust `length`.
- Go same-struct field rules such as `eqfield` map to Rust names such as
  `eq_field`.
- Go cross-struct rules such as `eqcsfield` do not get a second Rust rule name;
  existing `eq_field`-style rules accept relative dotted targets such as
  `profile.email`.
- Rust also has `range` and `regex`, which are not direct Go default-rule names.
- `omitempty` is supported as flow control in Rust attributes and expressions,
  even though it is not listed in Go `bakedInValidators`.

## Done

Required, default, size, and comparison:

- `required`
- `length` for Go `len`
- `min`
- `max`
- `range` as a Rust convenience rule
- `eq`
- `ne`
- `eq_ignore_case`
- `ne_ignore_case`
- `lt`
- `lte`
- `gt`
- `gte`
- `isdefault`

Same-struct and downward nested field comparison:

- `eq_field` for Go `eqfield`
- `ne_field` for Go `nefield`
- `gt_field` for Go `gtfield`
- `gte_field` for Go `gtefield`
- `lt_field` for Go `ltfield`
- `lte_field` for Go `ltefield`
- `fieldcontains`
- `fieldexcludes`

Single-segment targets remain siblings. Dotted targets are resolved relative
to the current struct or Schema object, without registering Go `*csfield`
compatibility names.

Conditional required and exclusion:

- `required_if`
- `required_unless`
- `skip_unless`
- `required_with`
- `required_with_all`
- `required_without`
- `required_without_all`
- `excluded_if`
- `excluded_unless`
- `excluded_with`
- `excluded_with_all`
- `excluded_without`
- `excluded_without_all`

String and character rules:

- `alpha`
- `alphanum`
- `boolean`
- `numeric`
- `number`
- `hexadecimal`
- `contains`
- `containsany`
- `containsrune`
- `excludes`
- `excludesall`
- `excludesrune`
- `startswith`
- `endswith`
- `startsnotwith`
- `endsnotwith`
- `ascii`
- `printascii`
- `multibyte`
- `lowercase`
- `uppercase`
- `alphaspace`
- `alphanumspace`
- `alphaunicode`
- `alphanumunicode`

Color and common format rules:

- `hexcolor`
- `rgb`
- `rgba`
- `hsl`
- `hsla`
- `cmyk`
- `e164`
- `email`
- `url`
- `http`
- `https`
- `uri`
- `base32`
- `base64`
- `base64url`
- `base64rawurl`
- `html`
- `html_encoded`
- `url_encoded`
- `json`
- `jwt`
- `datetime`
- `semver`
- `origin`
- `datauri`
- `latitude`
- `longitude`
- `ssn`
- `md4`
- `md5`
- `sha256`
- `sha384`
- `sha512`
- `ripemd128`
- `ripemd160`
- `tiger128`
- `tiger160`
- `tiger192`
- `eth_addr`
- `mongodb`
- `mongodb_connection_string`
- `dns_rfc1035_label`
- `cve`
- `cron`
- `ein`
- `bic_iso_9362_2014`
- `bic`
- `isbn`
- `isbn10`
- `isbn13`
- `issn`
- `credit_card`
- `luhn_checksum`

Network and identifiers:

- `ipv4`
- `ipv6`
- `ip`
- `cidrv4`
- `cidrv6`
- `cidr`
- `mac`
- `hostname`
- `hostname_port`
- `hostname_rfc1123`
- `fqdn`
- `port`
- `uuid`
- `uuid3`
- `uuid4`
- `uuid5`
- `uuid_rfc4122`
- `uuid3_rfc4122`
- `uuid4_rfc4122`
- `uuid5_rfc4122`
- `ulid`
- `tcp4`
- `tcp6`
- `tcp`
- `udp4`
- `udp6`
- `udp`

Collection and choice:

- `unique` without a field parameter
- single-field, compound-field, and nested-path projection for Vec, arrays, and slices of structs
- `oneof`
- `oneofci`
- `noneof`
- `noneofci`

Aliases:

- `iscolor`

Removed compatibility names:

- `ip4_addr`: use `ipv4`
- `ip6_addr`: use `ipv6`
- `ip_addr`: use `ip`
- `http_url`: use `http`
- `https_url`: use `https`
- `tcp4_addr`: use `tcp4`
- `tcp6_addr`: use `tcp6`
- `tcp_addr`: use `tcp`
- `udp4_addr`: use `udp4`
- `udp6_addr`: use `udp6`
- `udp_addr`: use `udp`

These names intentionally return `Error::UnknownRule`; the project does not
register compatibility aliases.

## Next

No pure validation-only batch remains after the current implementation pass.
Remaining items are in `Design`, `Later`, or `Won't Do In Core` because they
need API decisions, filesystem or OS state, external parser dependencies,
crypto/checksum dependencies, or standards-data maintenance.

## Design

These rules are likely useful, but should not be added until their API shape is
settled.

Parameterized parser rule:

- `spicedb`

Why: Go changes behavior based on param values `id`, `permission`, and `type`.
This is implementable, but should be designed as `#[validate(spicedb)]` and
`#[validate(spicedb(value = "permission"))]` or with a clearer parameter name.

Date/time compatibility:

- Go layout-parameter compatibility for `datetime` is intentionally excluded.

Rust keeps `datetime` as string format validation and uses `SystemTime` for
native time-point comparison. It does not add a second Go-layout parsing API.

## Later

These are technically possible, but not good immediate default-rule candidates.

External parser or dependency decisions:

- `urn_rfc2141`: Go uses `github.com/leodido/go-urn`.
- `bcp47_language_tag`: Go uses `golang.org/x/text/language.Parse`.
- `bcp47_strict_language_tag`: Go uses a large BCP47 regex plus extension parsing.
- `btc_addr`: Go validates Base58Check with SHA-256 checksum, not only regex.
- `btc_addr_bech32`: Go validates Bech32 checksum and witness program length.
- `eth_addr_checksum`: Go validates EIP-55 checksum using Keccak.

Filesystem and OS state:

- `file`: Go calls `os.Stat` and requires an existing non-directory.
- `filepath`: Go calls `os.Stat` and checks invalid path errors.
- `dir`: Go calls `os.Stat` and requires an existing directory.
- `dirpath`: Go calls `os.Stat` and checks path validity.
- `unix_addr`: Go calls `net.ResolveUnixAddr`.
- `uds_exists`: Go checks filesystem socket paths and Linux `/proc/net/unix`.
- `image`: Go reads a file and detects MIME type with `github.com/gabriel-vasile/mimetype`.
- `mimetype`: Go reads a file and detects MIME type with the same dependency.

Time zone database:

- `timezone`: Go uses `time.LoadLocation`, which depends on an available time zone
  database from the runtime, system, `ZONEINFO`, or embedded tzdata. Rust `std`
  does not ship an equivalent time zone database.

## Won't Do In Core

These require ongoing standards-data maintenance. They can live in an extension
crate later if needed, but not in the core default registry.

Country, currency, language, and postcode datasets:

- `iso3166_1_alpha2`
- `iso3166_1_alpha2_eu`
- `iso3166_1_alpha3`
- `iso3166_1_alpha3_eu`
- `iso3166_1_alpha_numeric`
- `iso3166_1_alpha_numeric_eu`
- `iso3166_2`
- `iso4217`
- `iso4217_numeric`
- `postcode_iso3166_alpha2`
- `postcode_iso3166_alpha2_field`
- alias `country_code`
- alias `eu_country_code`

Go-specific hook:

- `validateFn`

Why: this maps to Go method invocation patterns and does not directly translate
to Rust's trait-based model.

## Recommended Next Batch

The next implementation batch should start from explicit product value rather
than registry name parity. The remaining parser-rule design candidate is
parameterized `spicedb`, which requires an API decision before implementation.
