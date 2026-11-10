use halo::validator::prelude::*;
use serde::Serialize;
use serde_json::json;

struct Yielding;

impl Rule for Yielding {
    async fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        tokio::task::yield_now().await;
        Ok(field.value().string().is_some_and(|value| value == "ok"))
    }
}

#[derive(Serialize, Validate)]
struct Item {
    #[validate(yielding)]
    name: String,
}

#[derive(Validate)]
struct Container {
    #[validate(nested)]
    item: Item,
}

fn assert_send<T: Send>(_: T) {}

#[tokio::test(flavor = "multi_thread")]
async fn validation_futures_can_move_between_threads() {
    let validator = Validator::new().rule("yielding", Yielding).unwrap();
    let value = Container {
        item: Item { name: "ok".into() },
    };
    assert_send(validator.validate(&value));
    assert_send(validator.value(&"ok", "yielding"));
    assert_send(validator.partial(&value, ["item.name"]));
    assert_send(validator.except(&value, ["item.name"]));
    assert_send(validator.filter(&value, |_| true));
    tokio::spawn(async move {
        validator.validate(&value).await.unwrap();
        validator.partial(&value, ["item.name"]).await.unwrap();
        validator.except(&value, ["item.name"]).await.unwrap();
        validator.filter(&value, |_| true).await.unwrap();
        validator.value(&"ok", "yielding").await.unwrap();
    })
    .await
    .unwrap();

    let schema =
        Schema::from_yaml("fields:\n  name:\n    type: string\n    rules:\n      - yielding\n")
            .unwrap();
    let validator = Validator::with_schema(schema)
        .rule("yielding", Yielding)
        .unwrap();
    let data = json!({"name": "ok"});
    let item = Item { name: "ok".into() };
    assert_send(validator.check_map(&data));
    assert_send(validator.check_serde(&item));
    tokio::spawn(async move {
        validator.check_map(&data).await.unwrap();
        validator.check_serde(&item).await.unwrap();
    })
    .await
    .unwrap();
}
