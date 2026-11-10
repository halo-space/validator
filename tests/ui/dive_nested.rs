use halo::validator::prelude::*;

#[derive(Debug, Validate)]
struct Item {
    #[validate(required)]
    name: String,
}

#[derive(Debug, Validate)]
struct Basket {
    #[validate(dive(nested))]
    items: Vec<Item>,
}

#[tokio::main]
async fn main() {
    let basket = Basket {
        items: vec![Item {
            name: "validator".to_owned(),
        }],
    };

    Validator::new().validate(&basket).await.unwrap();
}
