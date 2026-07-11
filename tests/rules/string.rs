
#[derive(Debug, Validate)]
struct TextHelpers {
    #[validate(contains(value = "rust"))]
    body: String,

    #[validate(containsany(value = "!@#?"))]
    password: String,

    #[validate(containsrune(value = "好"))]
    greeting: String,

    #[validate(excludes(value = "admin"))]
    display_name: String,

    #[validate(excludesall(value = "!@#"))]
    slug: String,

    #[validate(excludesrune(value = "☻"))]
    mood: String,

    #[validate(startswith(value = "usr_"))]
    username: String,

    #[validate(endswith(value = ".rs"))]
    path: String,

    #[validate(startsnotwith(value = "tmp"))]
    storage_key: String,

    #[validate(endsnotwith(value = ".bak"))]
    filename: String,
}

#[test]
fn string_helpers_pass() {
    let value = TextHelpers {
        body: "hello rust".to_owned(),
        password: "hello!".to_owned(),
        greeting: "你好".to_owned(),
        display_name: "alice".to_owned(),
        slug: "hello-rust".to_owned(),
        mood: "happy".to_owned(),
        username: "usr_alice".to_owned(),
        path: "main.rs".to_owned(),
        storage_key: "cache/file".to_owned(),
        filename: "report.txt".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn string_helpers_fail() {
    let value = TextHelpers {
        body: "hello go".to_owned(),
        password: "hello".to_owned(),
        greeting: "hello".to_owned(),
        display_name: "root-admin".to_owned(),
        slug: "hello@rust".to_owned(),
        mood: "a☻b".to_owned(),
        username: "admin".to_owned(),
        path: "main.go".to_owned(),
        storage_key: "tmp/file".to_owned(),
        filename: "report.bak".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec![
            "contains",
            "containsany",
            "containsrune",
            "excludes",
            "excludesall",
            "excludesrune",
            "startswith",
            "endswith",
            "startsnotwith",
            "endsnotwith",
        ]
    );
    assert_eq!(fields[1].params().text("value"), Some("!@#?"));
}

#[derive(Debug, Validate)]
struct CharacterClasses {
    #[validate(ascii)]
    ascii: String,

    #[validate(printascii)]
    printascii: String,

    #[validate(multibyte)]
    multibyte: String,

    #[validate(alpha)]
    alpha: String,

    #[validate(alphanum)]
    alphanum: String,

    #[validate(numeric)]
    numeric: String,

    #[validate(number)]
    number: String,

    #[validate(lowercase)]
    lowercase: String,

    #[validate(uppercase)]
    uppercase: String,

    #[validate(boolean)]
    boolean: String,
}

#[test]
fn character_classes_pass() {
    let value = CharacterClasses {
        ascii: "abc-123".to_owned(),
        printascii: "Hello, Rust!".to_owned(),
        multibyte: "你好".to_owned(),
        alpha: "abcXYZ".to_owned(),
        alphanum: "abc123".to_owned(),
        numeric: "-12.5".to_owned(),
        number: "12345".to_owned(),
        lowercase: "rust".to_owned(),
        uppercase: "RUST".to_owned(),
        boolean: "true".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn character_classes_fail() {
    let value = CharacterClasses {
        ascii: "你好".to_owned(),
        printascii: "hello\n".to_owned(),
        multibyte: "hello".to_owned(),
        alpha: "abc1".to_owned(),
        alphanum: "abc-123".to_owned(),
        numeric: "12e3".to_owned(),
        number: "12.3".to_owned(),
        lowercase: "Rust".to_owned(),
        uppercase: "Rust".to_owned(),
        boolean: "maybe".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 10);
    assert_eq!(fields[0].rule(), "ascii");
    assert_eq!(fields[1].rule(), "printascii");
    assert_eq!(fields[2].rule(), "multibyte");
    assert_eq!(fields[3].rule(), "alpha");
    assert_eq!(fields[4].rule(), "alphanum");
    assert_eq!(fields[5].rule(), "numeric");
    assert_eq!(fields[6].rule(), "number");
    assert_eq!(fields[7].rule(), "lowercase");
    assert_eq!(fields[8].rule(), "uppercase");
    assert_eq!(fields[9].rule(), "boolean");
}

#[derive(Debug, Validate)]
struct ExtendedCharacterClasses {
    #[validate(alphaspace)]
    alphaspace: String,

    #[validate(alphanumspace)]
    alphanumspace: String,

    #[validate(alphaunicode)]
    alphaunicode: String,

    #[validate(alphanumunicode)]
    alphanumunicode: String,

    #[validate(eq_ignore_case = "Rust")]
    exact: String,

    #[validate(ne_ignore_case = "admin")]
    username: String,
}

#[test]
fn extended_character_classes_pass() {
    let value = ExtendedCharacterClasses {
        alphaspace: "Hello Rust".to_owned(),
        alphanumspace: "Rust 2024".to_owned(),
        alphaunicode: "你好Rust".to_owned(),
        alphanumunicode: "你好Rust2024".to_owned(),
        exact: "rust".to_owned(),
        username: "alice".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn extended_character_classes_fail() {
    let value = ExtendedCharacterClasses {
        alphaspace: "Hello 2024".to_owned(),
        alphanumspace: "Rust-2024".to_owned(),
        alphaunicode: "你好2024".to_owned(),
        alphanumunicode: "你好_Rust".to_owned(),
        exact: "Go".to_owned(),
        username: "ADMIN".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec![
            "alphaspace",
            "alphanumspace",
            "alphaunicode",
            "alphanumunicode",
            "eq_ignore_case",
            "ne_ignore_case",
        ]
    );
    assert_eq!(fields[4].params().text("value"), Some("Rust"));
}
