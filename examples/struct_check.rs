use validator::prelude::*;

#[derive(Debug, Validate)]
struct Event {
    start_at: i64,

    #[validate(gt_field = "start_at")]
    end_at: i64,
}

#[derive(Debug, Validate)]
#[validate(check = "check_draft")]
struct Draft {
    name: String,
    title: String,
}

fn check_draft(draft: &Draft, valid: &mut validator::valid::Valid<'_>) {
    if draft.name.is_empty() && draft.title.is_empty() {
        valid
            .field("name")
            .rule("required_without")
            .param("field", "title")
            .push();
    }
}

fn main() -> Result<(), Error> {
    let validator = Validator::new();
    validator.validate(&Event {
        start_at: 10,
        end_at: 20,
    })?;

    let error = validator
        .validate(&Draft {
            name: String::new(),
            title: String::new(),
        })
        .unwrap_err();
    let fields = error
        .fields()
        .expect("validation errors must contain fields");
    assert_eq!(fields[0].rule(), "required_without");

    Ok(())
}
