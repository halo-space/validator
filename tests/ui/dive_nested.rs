use validator::prelude::*;

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

fn main() {
    let basket = Basket {
        items: vec![Item {
            name: "validator".to_owned(),
        }],
    };

    Validator::new().validate(&basket).unwrap();
}
