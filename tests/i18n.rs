use validator::prelude::*;

fn fields(error: validator::Error) -> Vec<validator::FieldError> {
    error.into_fields().expect("expected validation errors")
}

#[derive(Debug, Validate)]
struct Account {
    #[validate(required, email)]
    email: String,
}

#[test]
fn zh_cn_renders_messages_and_keeps_context() {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());

    let messages = validator::i18n::zh_cn().render(&fields);

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "email格式不正确");
    assert_eq!(messages[0].namespace.as_str(), "Account.email");
    assert_eq!(messages[0].field, "email");
    assert_eq!(messages[0].rule, "email");
    assert_eq!(messages[0].reason, "email");
    assert_eq!(messages[0].kind, Kind::String);
}

#[test]
fn en_renders_messages() {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());

    let messages = validator::i18n::en().render(&fields);

    assert_eq!(messages[0].text, "email must be a valid email address");
}

#[test]
fn dynamic_locale_selection_uses_fallback() {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());
    let i18n = validator::i18n::new().zh_cn().en().fallback("zh-CN");

    let en = i18n.locale("en").render(&fields);
    let fr = i18n.locale("fr").render(&fields);

    assert_eq!(en[0].text, "email must be a valid email address");
    assert_eq!(fr[0].text, "email格式不正确");
}

#[test]
fn missing_locale_and_fallback_still_render_default_text() {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());
    let i18n = validator::i18n::new().fallback("zh-CN");

    let messages = i18n.locale("fr").render(&fields);

    assert_eq!(messages[0].text, "Account.email failed email");
}

#[test]
fn user_locale_overrides_rule_and_field_label() {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());
    let zh = validator::i18n::Locale::new("zh-CN")
        .rule("email", "请输入正确的{field}")
        .field("email", "邮箱");
    let i18n = validator::i18n::new()
        .zh_cn()
        .use_locale(zh)
        .fallback("zh-CN");

    let messages = i18n.locale("zh-CN").render(&fields);

    assert_eq!(messages[0].field, "邮箱");
    assert_eq!(messages[0].text, "请输入正确的邮箱");
}

#[test]
fn yaml_locale_renders_messages() -> Result<(), Box<dyn std::error::Error>> {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());
    let zh = validator::i18n::Locale::from_yaml(
        r#"
locale: zh-CN
rules:
  email: "请输入正确的{field}"
fields:
  email: "邮箱"
"#,
    )?;

    let messages = validator::i18n::new()
        .use_locale(zh)
        .locale("zh-CN")
        .render(&fields);

    assert_eq!(messages[0].field, "邮箱");
    assert_eq!(messages[0].text, "请输入正确的邮箱");

    Ok(())
}

#[test]
fn json_locale_renders_messages() -> Result<(), Box<dyn std::error::Error>> {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());
    let en = validator::i18n::Locale::from_json(
        r#"{
  "locale": "en",
  "rules": {
    "email": "Please use a valid {field}"
  },
  "fields": {
    "email": "email address"
  }
}"#,
    )?;

    let messages = validator::i18n::new()
        .use_locale(en)
        .locale("en")
        .render(&fields);

    assert_eq!(messages[0].field, "email address");
    assert_eq!(messages[0].text, "Please use a valid email address");

    Ok(())
}

#[test]
fn locale_loader_rejects_name_alias() {
    let Err(error) = validator::i18n::Locale::from_yaml(
        r#"
name: zh-CN
rules:
  email: "{field}格式不正确"
"#,
    ) else {
        panic!("expected locale loading to fail");
    };

    assert!(matches!(
        error,
        validator::Error::InvalidData { reason }
            if reason.contains("unknown field `name`")
                || reason.contains("unknown field 'name'")
    ));
}

#[test]
fn loaded_locale_overrides_builtin_locale() -> Result<(), Box<dyn std::error::Error>> {
    let account = Account {
        email: "not-email".to_owned(),
    };
    let fields = fields(Validator::new().validate(&account).unwrap_err());
    let zh = validator::i18n::Locale::from_yaml(
        r#"
locale: zh-CN
rules:
  email: "邮箱要写对"
"#,
    )?;

    let messages = validator::i18n::new()
        .zh_cn()
        .use_locale(zh)
        .locale("zh-CN")
        .render(&fields);

    assert_eq!(messages[0].text, "邮箱要写对");

    Ok(())
}

#[test]
fn locale_loader_requires_locale_name() {
    let Err(error) = validator::i18n::Locale::from_yaml(
        r#"
rules:
  email: "{field}格式不正确"
"#,
    ) else {
        panic!("expected locale loading to fail");
    };

    assert!(matches!(
        error,
        validator::Error::InvalidData { reason }
            if reason.contains("invalid locale resource")
                && reason.contains("locale name is required")
    ));
}

#[test]
fn locale_loader_rejects_invalid_rule_template() {
    let Err(error) = validator::i18n::Locale::from_yaml(
        r#"
locale: zh-CN
rules:
  email:
    text: "{field}格式不正确"
"#,
    ) else {
        panic!("expected locale loading to fail");
    };

    assert!(matches!(
        error,
        validator::Error::InvalidData { reason }
            if reason.contains("invalid locale resource")
    ));
}

#[test]
fn locale_loader_rejects_unknown_key() {
    let Err(error) = validator::i18n::Locale::from_yaml(
        r#"
locale: zh-CN
rulse:
  email: "{field}格式不正确"
"#,
    ) else {
        panic!("expected locale loading to fail");
    };

    assert!(matches!(
        error,
        validator::Error::InvalidData { reason }
            if reason.contains("unknown field `rulse`")
    ));
}

#[derive(Debug, Validate)]
struct Profile {
    #[validate(alias = "username")]
    name: String,
}

#[test]
fn alias_template_is_checked_before_reason_template() -> Result<(), Box<dyn std::error::Error>> {
    let profile = Profile {
        name: "ab".to_owned(),
    };
    let fields = fields(
        Validator::new()
            .alias("username", "required,length(min=3,max=20)")?
            .validate(&profile)
            .unwrap_err(),
    );
    let zh = validator::i18n::Locale::new("zh-CN")
        .rule("username", "用户名不合法")
        .rule("length", "{field}长度必须在{min}到{max}之间");

    let messages = validator::i18n::new()
        .use_locale(zh)
        .locale("zh-CN")
        .render(&fields);

    assert_eq!(messages[0].text, "用户名不合法");
    assert_eq!(messages[0].rule, "username");
    assert_eq!(messages[0].reason, "length");

    Ok(())
}

#[test]
fn alias_falls_back_to_reason_template() -> Result<(), Box<dyn std::error::Error>> {
    let profile = Profile {
        name: "ab".to_owned(),
    };
    let fields = fields(
        Validator::new()
            .alias("username", "required,length(min=3,max=20)")?
            .validate(&profile)
            .unwrap_err(),
    );

    let messages = validator::i18n::zh_cn().render(&fields);

    assert_eq!(messages[0].text, "name长度必须在3到20之间");

    Ok(())
}

#[test]
fn nested_alias_uses_outer_template_and_actual_reason() -> Result<(), Box<dyn std::error::Error>> {
    let fields = fields(
        Validator::new()
            .alias("inner", "email")?
            .alias("outer", "inner")?
            .value(&"invalid", "outer")
            .unwrap_err(),
    );
    let locale = validator::i18n::Locale::new("en").rule("outer", "outer failed {reason}");
    let messages = validator::i18n::new()
        .use_locale(locale)
        .locale("en")
        .render(&fields);

    assert_eq!(messages[0].rule, "outer");
    assert_eq!(messages[0].reason, "email");
    assert_eq!(messages[0].text, "outer failed email");
    Ok(())
}

#[derive(Debug, Validate)]
struct ColorAlias {
    #[validate(alias = "iscolor")]
    color: String,
}

#[test]
fn built_in_locales_render_iscolor_alias_message() {
    let value = ColorAlias {
        color: "#000-".to_owned(),
    };
    let fields = fields(Validator::new().validate(&value).unwrap_err());

    let zh = validator::i18n::zh_cn().render(&fields);
    let en = validator::i18n::en().render(&fields);

    assert_eq!(fields[0].rule(), "iscolor");
    assert_eq!(fields[0].reason(), "hexcolor|rgb|rgba|hsl|hsla|cmyk");
    assert_eq!(zh[0].text, "color必须是有效颜色");
    assert_eq!(en[0].text, "color must be a valid color");
}

#[derive(Debug, Validate)]
struct Bounds {
    #[validate(min = 3)]
    name: String,
}

#[test]
fn function_template_can_read_param_and_kind() {
    let bounds = Bounds {
        name: "ab".to_owned(),
    };
    let fields = fields(Validator::new().validate(&bounds).unwrap_err());
    let en = validator::i18n::Locale::new("en").rule_fn("min", |ctx| {
        assert_eq!(ctx.kind(), Kind::String);
        format!("{} needs {} chars", ctx.field(), ctx.param("min").unwrap())
    });

    let messages = validator::i18n::new()
        .use_locale(en)
        .locale("en")
        .render(&fields);

    assert_eq!(messages[0].text, "name needs 3 chars");
}

#[derive(Debug)]
struct LocalizedMember {
    tenant_id: u64,
    email: String,
}

#[derive(Debug, Validate)]
struct LocalizedTeam {
    #[validate(unique = ["tenant_id", "email"])]
    members: Vec<LocalizedMember>,
}

#[test]
fn function_template_can_read_compound_unique_fields() {
    let fields = fields(
        Validator::new()
            .validate(&LocalizedTeam {
                members: vec![
                    LocalizedMember {
                        tenant_id: 1,
                        email: "same@example.com".to_owned(),
                    },
                    LocalizedMember {
                        tenant_id: 1,
                        email: "same@example.com".to_owned(),
                    },
                ],
            })
            .unwrap_err(),
    );
    let en = validator::i18n::Locale::new("en").rule_fn("unique", |ctx| {
        format!(
            "{} must be unique by {}",
            ctx.field(),
            ctx.param_list("fields").unwrap().join(" + ")
        )
    });

    let messages = validator::i18n::new()
        .use_locale(en)
        .locale("en")
        .render(&fields);

    assert_eq!(
        messages[0].text,
        "members must be unique by tenant_id + email"
    );
}

#[test]
fn template_substitutions_are_not_rendered_twice() {
    let fields = fields(
        Validator::new()
            .value(&"actual", r#"eq(value="{field}")"#)
            .unwrap_err(),
    );
    let locale = validator::i18n::Locale::new("en")
        .rule("eq", "{field} expected {value} ({unknown})")
        .field("$value", "{rule}");

    let messages = validator::i18n::new()
        .use_locale(locale)
        .locale("en")
        .render(&fields);

    assert_eq!(messages[0].text, "{rule} expected {field} ({unknown})");
}

#[derive(Debug, Validate)]
struct NewRuleMessages {
    #[validate(eq = "published")]
    state: String,

    #[validate(https)]
    source_url: String,

    #[validate(cidr)]
    network: String,

    #[validate(hostname_rfc1123)]
    rfc_host: String,

    #[validate(unique)]
    tags: Vec<String>,

    #[validate(ulid)]
    public_id: String,

    #[validate(ascii)]
    code: String,

    #[validate(containsany(value = "!@#?"))]
    password: String,

    #[validate(noneof("root", "admin"))]
    username: String,
}

#[test]
fn i18n_renders_new_rule_messages() {
    let value = NewRuleMessages {
        state: "draft".to_owned(),
        source_url: "http://example.com".to_owned(),
        network: "10.0.0.0/33".to_owned(),
        rfc_host: "foo.bar:80".to_owned(),
        tags: vec!["rust".to_owned(), "rust".to_owned()],
        public_id: "01BX5ZZKBKACTAV9WEVGEMMVRU".to_owned(),
        code: "你好".to_owned(),
        password: "hello".to_owned(),
        username: "root".to_owned(),
    };
    let fields = fields(Validator::new().validate(&value).unwrap_err());

    let messages = validator::i18n::en().render(&fields);

    assert_eq!(messages[0].text, "state must be equal to published");
    assert_eq!(messages[1].text, "source_url must be a valid HTTPS URL");
    assert_eq!(messages[2].text, "network must be a valid CIDR block");
    assert_eq!(
        messages[3].text,
        "rfc_host must be a valid RFC1123 hostname"
    );
    assert_eq!(messages[4].text, "tags must contain unique values");
    assert_eq!(messages[5].text, "public_id must be a valid ULID");
    assert_eq!(messages[6].text, "code must contain only ASCII characters");
    assert_eq!(messages[7].text, "password must contain any of: !@#?");
    assert_eq!(messages[8].text, "username must not be one of: root,admin");
}

#[derive(Debug, Validate)]
struct CrossFieldMessages {
    password: String,

    #[validate(eq_field = "password")]
    confirm_password: String,
}

#[test]
fn i18n_renders_cross_field_messages() {
    let value = CrossFieldMessages {
        password: "secret".to_owned(),
        confirm_password: "different".to_owned(),
    };
    let fields = fields(Validator::new().validate(&value).unwrap_err());

    let zh = validator::i18n::zh_cn().render(&fields);
    let en = validator::i18n::en().render(&fields);

    assert_eq!(zh[0].text, "confirm_password必须等于password");
    assert_eq!(en[0].text, "confirm_password must be equal to password");
}
