use halo::validator::prelude::*;

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

async fn check_draft(draft: &Draft, valid: &mut halo::validator::valid::Valid<'_>) {
    if draft.name.is_empty() && draft.title.is_empty() {
        valid
            .field("name")
            .rule("required_without")
            .param("field", "title")
            .push();
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let validator = Validator::new();
    validator
        .validate(&Event {
            start_at: 10,
            end_at: 20,
        })
        .await?;

    let error = validator
        .validate(&Draft {
            name: String::new(),
            title: String::new(),
        })
        .await
        .unwrap_err();
    let fields = error
        .fields()
        .expect("validation errors must contain fields");
    assert_eq!(fields[0].rule(), "required_without");

    Ok(())
}
