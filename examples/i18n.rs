use validator::i18n::{self, Locale};
use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required)]
    email: String,
}

fn main() -> Result<(), Error> {
    let error = Validator::new()
        .validate(&User {
            email: String::new(),
        })
        .unwrap_err();
    let fields = error
        .fields()
        .expect("validation errors must contain fields");

    let zh_cn = i18n::zh_cn().render(fields);
    let en = i18n::en().render(fields);
    assert!(!zh_cn[0].text.is_empty());
    assert!(!en[0].text.is_empty());

    let catalog = i18n::new().use_locale(
        Locale::new("custom")
            .field("email", "Email address")
            .rule("required", "{field} is missing"),
    );
    let messages = catalog.locale("custom").render(fields);
    assert_eq!(messages[0].text, "Email address is missing");

    Ok(())
}
