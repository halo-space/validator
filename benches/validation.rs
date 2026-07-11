use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use serde::Serialize;
use validator::prelude::*;

const DIRECT_EXPRESSION: &str = "required,email,length(min=6,max=254)";

const SCHEMA: &str = r#"
fields:
  name:
    type: string
    rules: required,length(min=3,max=32)
  email:
    type: string
    rules: required,email
  age:
    type: uint
    rules: gte=18,lte=130
  profile:
    type: object
    fields:
      website:
        type: string
        rules: omitempty,url
"#;

#[derive(Debug, Validate)]
struct Profile {
    #[validate(omitempty, url)]
    website: String,
}

#[derive(Debug, Validate)]
struct User {
    #[validate(required, length(min = 3, max = 32))]
    name: String,

    #[validate(required, email)]
    email: String,

    #[validate(gte = 18, lte = 130)]
    age: u8,

    #[validate(nested)]
    profile: Profile,

    #[validate(dive(required, email))]
    contacts: Vec<String>,
}

#[derive(Serialize)]
struct SchemaProfile {
    website: String,
}

#[derive(Serialize)]
struct SchemaUser {
    name: String,
    email: String,
    age: u8,
    profile: SchemaProfile,
}

struct MemberProfile {
    email: String,
}

struct Member {
    tenant_id: u64,
    profile: MemberProfile,
}

#[derive(Validate)]
struct Team {
    #[validate(unique = ["tenant_id", "profile.email"])]
    members: Vec<Member>,
}

#[derive(Validate)]
struct ContactList {
    #[validate(dive(required, email))]
    emails: Vec<String>,
}

fn valid_user() -> User {
    User {
        name: "Xiaohan".to_owned(),
        email: "xiaohan@example.com".to_owned(),
        age: 30,
        profile: Profile {
            website: "https://example.com/profile".to_owned(),
        },
        contacts: vec![
            "first@example.com".to_owned(),
            "second@example.com".to_owned(),
            "third@example.com".to_owned(),
        ],
    }
}

fn invalid_user() -> User {
    User {
        name: "x".to_owned(),
        email: "invalid".to_owned(),
        age: 10,
        profile: Profile {
            website: "not-a-url".to_owned(),
        },
        contacts: vec!["invalid".to_owned(), String::new()],
    }
}

fn schema_user() -> SchemaUser {
    SchemaUser {
        name: "Xiaohan".to_owned(),
        email: "xiaohan@example.com".to_owned(),
        age: 30,
        profile: SchemaProfile {
            website: "https://example.com/profile".to_owned(),
        },
    }
}

fn schema_data() -> serde_json::Value {
    serde_json::to_value(schema_user()).expect("benchmark data must serialize")
}

fn schema_validator() -> Validator {
    Validator::with_schema(Schema::from_yaml(SCHEMA).expect("benchmark schema must compile"))
}

fn contact_list(count: usize) -> ContactList {
    ContactList {
        emails: (0..count)
            .map(|index| format!("user{index}@example.com"))
            .collect(),
    }
}

fn team(count: usize) -> Team {
    Team {
        members: (0..count)
            .map(|index| Member {
                tenant_id: (index % 10) as u64,
                profile: MemberProfile {
                    email: format!("user{index}@example.com"),
                },
            })
            .collect(),
    }
}

fn derive_validation(criterion: &mut Criterion) {
    let validator = Validator::new();
    let valid = valid_user();
    let invalid = invalid_user();
    let mut group = criterion.benchmark_group("derive");

    group.bench_function("valid", |bench| {
        bench.iter(|| {
            validator
                .validate(black_box(&valid))
                .expect("valid benchmark data")
        });
    });
    group.bench_function("invalid", |bench| {
        bench.iter(|| {
            black_box(
                validator
                    .validate(black_box(&invalid))
                    .expect_err("invalid benchmark data"),
            );
        });
    });
    group.finish();
}

fn direct_value_validation(criterion: &mut Criterion) {
    let email = "xiaohan@example.com".to_owned();
    let validator = Validator::new();
    validator
        .value(&email, DIRECT_EXPRESSION)
        .expect("warm-up value must pass");

    let mut group = criterion.benchmark_group("direct_value");
    group.bench_function("warm", |bench| {
        bench.iter(|| {
            validator
                .value(black_box(&email), black_box(DIRECT_EXPRESSION))
                .expect("benchmark value must pass");
        });
    });
    group.bench_function("cold_compile", |bench| {
        bench.iter_batched(
            Validator::new,
            |validator| {
                validator
                    .value(black_box(&email), black_box(DIRECT_EXPRESSION))
                    .expect("benchmark value must pass");
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn selective_validation(criterion: &mut Criterion) {
    let validator = Validator::new();
    let user = valid_user();
    let mut group = criterion.benchmark_group("selective");

    group.bench_function("partial", |bench| {
        bench.iter(|| {
            validator
                .partial(
                    black_box(&user),
                    black_box(["profile.website", "contacts[0]"]),
                )
                .expect("selected benchmark fields must pass");
        });
    });
    group.bench_function("except", |bench| {
        bench.iter(|| {
            validator
                .except(black_box(&user), black_box(["profile.website"]))
                .expect("remaining benchmark fields must pass");
        });
    });
    group.bench_function("filter", |bench| {
        bench.iter(|| {
            validator
                .filter(black_box(&user), |namespace| {
                    matches!(namespace.as_str(), "profile" | "profile.website")
                })
                .expect("filtered benchmark fields must pass");
        });
    });
    group.finish();
}

fn schema_validation(criterion: &mut Criterion) {
    let data = schema_data();
    let serializable = schema_user();
    let validator = schema_validator();
    validator
        .validate_map(&data)
        .expect("warm-up schema data must pass");

    let mut group = criterion.benchmark_group("schema");
    group.bench_function("map_warm", |bench| {
        bench.iter(|| {
            validator
                .validate_map(black_box(&data))
                .expect("benchmark schema data must pass");
        });
    });
    group.bench_function("serde_warm", |bench| {
        bench.iter(|| {
            validator
                .validate_serde(black_box(&serializable))
                .expect("benchmark serializable data must pass");
        });
    });
    group.bench_function("map_cold_compile", |bench| {
        bench.iter_batched(
            schema_validator,
            |validator| {
                validator
                    .validate_map(black_box(&data))
                    .expect("benchmark schema data must pass");
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn collection_validation(criterion: &mut Criterion) {
    const COUNT: usize = 100;

    let validator = Validator::new();
    let contacts = contact_list(COUNT);
    let team = team(COUNT);
    let mut group = criterion.benchmark_group("collections");
    group.throughput(Throughput::Elements(COUNT as u64));

    group.bench_function("dive_email", |bench| {
        bench.iter(|| {
            validator
                .validate(black_box(&contacts))
                .expect("benchmark contacts must pass");
        });
    });
    group.bench_function("compound_unique", |bench| {
        bench.iter(|| {
            validator
                .validate(black_box(&team))
                .expect("benchmark team must pass");
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    derive_validation,
    direct_value_validation,
    selective_validation,
    schema_validation,
    collection_validation
);
criterion_main!(benches);
