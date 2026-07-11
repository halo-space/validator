use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
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

fn invalid_schema_user() -> SchemaUser {
    SchemaUser {
        name: "x".to_owned(),
        email: "invalid".to_owned(),
        age: 10,
        profile: SchemaProfile {
            website: "not-a-url".to_owned(),
        },
    }
}

fn schema_data() -> serde_json::Value {
    serde_json::to_value(schema_user()).expect("benchmark data must serialize")
}

fn invalid_schema_data() -> serde_json::Value {
    serde_json::to_value(invalid_schema_user()).expect("benchmark data must serialize")
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
    validator
        .validate(&valid)
        .expect("warm-up derive data must pass");
    validator
        .validate(&invalid)
        .expect_err("warm-up derive data must fail");
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
    let invalid_email = "invalid".to_owned();
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
    group.bench_function("invalid_warm", |bench| {
        bench.iter(|| {
            black_box(
                validator
                    .value(black_box(&invalid_email), black_box(DIRECT_EXPRESSION))
                    .expect_err("benchmark value must fail"),
            );
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
                .partial(black_box(&user), black_box(["profile.website", "email"]))
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
    let invalid_data = invalid_schema_data();
    let serializable = schema_user();
    let invalid_serializable = invalid_schema_user();
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
    group.bench_function("map_invalid_warm", |bench| {
        bench.iter(|| {
            black_box(
                validator
                    .validate_map(black_box(&invalid_data))
                    .expect_err("benchmark schema data must fail"),
            );
        });
    });
    group.bench_function("serde_warm", |bench| {
        bench.iter(|| {
            validator
                .validate_serde(black_box(&serializable))
                .expect("benchmark serializable data must pass");
        });
    });
    group.bench_function("serde_invalid_warm", |bench| {
        bench.iter(|| {
            black_box(
                validator
                    .validate_serde(black_box(&invalid_serializable))
                    .expect_err("benchmark serializable data must fail"),
            );
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
    const COUNTS: [usize; 3] = [10, 100, 1_000];

    let validator = Validator::new();
    let mut dive = criterion.benchmark_group("collections/dive_email");
    for count in COUNTS {
        let contacts = contact_list(count);
        dive.throughput(Throughput::Elements(count as u64));
        dive.bench_with_input(
            BenchmarkId::from_parameter(count),
            &contacts,
            |bench, contacts| {
                bench.iter(|| {
                    validator
                        .validate(black_box(contacts))
                        .expect("benchmark contacts must pass");
                });
            },
        );
    }
    dive.finish();

    let mut unique = criterion.benchmark_group("collections/compound_unique");
    for count in COUNTS {
        let team = team(count);
        unique.throughput(Throughput::Elements(count as u64));
        unique.bench_with_input(BenchmarkId::from_parameter(count), &team, |bench, team| {
            bench.iter(|| {
                validator
                    .validate(black_box(team))
                    .expect("benchmark team must pass");
            });
        });
    }
    unique.finish();
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
