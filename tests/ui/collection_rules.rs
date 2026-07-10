use validator::prelude::*;

#[derive(Debug)]
struct Item {
    id: u64,
}

impl Value for Item {
    fn kind(&self) -> Kind {
        Kind::Uint(UintKind::U64)
    }

    fn required(&self) -> bool {
        self.id != 0
    }

    fn uint(&self) -> Option<u128> {
        Some(u128::from(self.id))
    }
}

#[derive(Debug, Validate)]
struct Basket {
    #[validate(required, min = 1, unique)]
    items: Vec<Item>,
}

fn main() {
    let basket = Basket {
        items: vec![Item { id: 1 }, Item { id: 2 }],
    };

    Validator::new().validate(&basket).unwrap();
}
