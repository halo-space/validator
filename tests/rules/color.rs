
#[derive(Debug, Validate)]
struct ColorFormats {
    #[validate(hexcolor)]
    hex: String,

    #[validate(rgb)]
    rgb: String,

    #[validate(rgba)]
    rgba: String,

    #[validate(hsl)]
    hsl: String,

    #[validate(hsla)]
    hsla: String,

    #[validate(cmyk)]
    cmyk: String,
}

#[test]
fn color_rules_pass() {
    let value = ColorFormats {
        hex: "#00ffaa".to_owned(),
        rgb: "rgb(255, 0, 120)".to_owned(),
        rgba: "rgba(255, 0, 120, 0.5)".to_owned(),
        hsl: "hsl(360, 100%, 50%)".to_owned(),
        hsla: "hsla(240, 100%, 50%, 1)".to_owned(),
        cmyk: "cmyk(0%, 10%, 20%, 100%)".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn color_rules_fail() {
    let value = ColorFormats {
        hex: "#000-".to_owned(),
        rgb: "rgb(256, 0, 0)".to_owned(),
        rgba: "rgba(0, 0, 0, 1.5)".to_owned(),
        hsl: "hsl(361, 100%, 50%)".to_owned(),
        hsla: "hsla(240, 100%, 50%, 2)".to_owned(),
        cmyk: "cmyk(0%, 10%, 20%, 101%)".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec!["hexcolor", "rgb", "rgba", "hsl", "hsla", "cmyk"]
    );
}

#[derive(Debug, Validate)]
struct FavoriteColor {
    #[validate(alias = "iscolor")]
    color: String,
}

#[test]
fn default_iscolor_alias_accepts_any_color_format() {
    let hex = FavoriteColor {
        color: "#fff".to_owned(),
    };
    let rgb = FavoriteColor {
        color: "rgb(255, 255, 255)".to_owned(),
    };

    Validator::new().validate(&hex).unwrap();
    Validator::new().validate(&rgb).unwrap();
}

#[test]
fn default_iscolor_alias_reports_alias_failure() {
    let value = FavoriteColor {
        color: "#000-".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "iscolor");
    assert_eq!(fields[0].reason(), "hexcolor|rgb|rgba|hsl|hsla|cmyk");
}
