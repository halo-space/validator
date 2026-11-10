use std::cell::RefCell;
use std::collections::BTreeMap;

use halo::validator::prelude::*;

#[derive(Validate)]
struct FlatUser {
    #[validate(required)]
    name: String,

    #[validate(required)]
    email: String,
}

#[derive(Validate)]
struct Profile {
    #[validate(required)]
    email: String,

    #[validate(required)]
    display_name: String,
}

#[derive(Validate)]
struct NestedUser {
    #[validate(required)]
    name: String,

    #[validate(nested)]
    profile: Profile,
}

#[derive(Validate)]
struct Member {
    #[validate(required)]
    email: String,

    #[validate(required)]
    name: String,
}

#[derive(Validate)]
struct Team {
    #[validate(dive(required, nested))]
    members: Vec<Member>,
}

#[derive(Validate)]
struct Labels {
    #[validate(dive(keys(required), values(required)))]
    values: BTreeMap<String, String>,
}

#[derive(Validate)]
#[validate(check = "check_pair")]
struct CheckedPair {
    left: String,
    right: String,
}

#[derive(Validate)]
struct RuntimeAlias {
    #[validate(alias = "missing_alias")]
    hidden: String,

    #[validate(required)]
    visible: String,
}

#[derive(Validate)]
#[validate(check = "check_nested_path")]
struct CheckedNestedPath {
    profile: Profile,
}

async fn check_pair(value: &CheckedPair, valid: &mut halo::validator::valid::Valid<'_>) {
    let _equal = value.left == value.right;
    valid.field("left").rule("pair").push();
    valid.field("right").rule("pair").push();
}

async fn check_nested_path(
    value: &CheckedNestedPath,
    valid: &mut halo::validator::valid::Valid<'_>,
) {
    let _ = &value.profile;
    valid.field("profile.email").rule("nested_pair").push();
}

fn namespaces(error: Error) -> Vec<String> {
    error
        .into_fields()
        .expect("validation failure must contain field errors")
        .into_iter()
        .map(|field| field.namespace().as_str().to_owned())
        .collect()
}

#[tokio::test]
async fn partial_and_except_select_top_level_fields() {
    let value = FlatUser {
        name: String::new(),
        email: String::new(),
    };

    let partial = Validator::new()
        .partial(&value, ["email"])
        .await
        .expect_err("selected email must fail");
    assert_eq!(namespaces(partial), ["FlatUser.email"]);

    let except = Validator::new()
        .except(&value, ["email"])
        .await
        .expect_err("non-excluded name must fail");
    assert_eq!(namespaces(except), ["FlatUser.name"]);
}

#[tokio::test]
async fn partial_selects_nested_field_or_complete_subtree() {
    let value = NestedUser {
        name: String::new(),
        profile: Profile {
            email: String::new(),
            display_name: String::new(),
        },
    };

    let field = Validator::new()
        .partial(&value, ["profile.email"])
        .await
        .expect_err("selected nested email must fail");
    assert_eq!(namespaces(field), ["NestedUser.profile.email"]);

    let subtree = Validator::new()
        .partial(&value, ["profile"])
        .await
        .expect_err("selected profile subtree must fail");
    assert_eq!(
        namespaces(subtree),
        [
            "NestedUser.profile.email",
            "NestedUser.profile.display_name"
        ]
    );
}

#[tokio::test]
async fn except_skips_only_selected_nested_subtree() {
    let value = NestedUser {
        name: String::new(),
        profile: Profile {
            email: String::new(),
            display_name: String::new(),
        },
    };

    let error = Validator::new()
        .except(&value, ["profile.email"])
        .await
        .expect_err("remaining fields must fail");
    assert_eq!(
        namespaces(error),
        ["NestedUser.name", "NestedUser.profile.display_name"]
    );
}

#[tokio::test]
async fn partial_selects_one_dive_index() {
    let value = Team {
        members: vec![
            Member {
                email: String::new(),
                name: String::new(),
            },
            Member {
                email: String::new(),
                name: String::new(),
            },
        ],
    };

    let error = Validator::new()
        .partial(&value, ["members[0].email"])
        .await
        .expect_err("selected collection field must fail");
    assert_eq!(namespaces(error), ["Team.members[0].email"]);
}

#[tokio::test]
async fn except_one_dive_field_keeps_other_indices() {
    let value = Team {
        members: vec![
            Member {
                email: String::new(),
                name: String::new(),
            },
            Member {
                email: String::new(),
                name: String::new(),
            },
        ],
    };

    let error = Validator::new()
        .except(&value, ["members[0].email"])
        .await
        .expect_err("remaining collection fields must fail");
    assert_eq!(
        namespaces(error),
        [
            "Team.members[0].name",
            "Team.members[1].email",
            "Team.members[1].name"
        ]
    );
}

#[tokio::test]
async fn partial_matches_map_entry_namespace() {
    let value = Labels {
        values: BTreeMap::from([
            ("other".to_owned(), String::new()),
            ("source".to_owned(), String::new()),
        ]),
    };

    let error = Validator::new()
        .partial(&value, [r#"values["source"]"#])
        .await
        .expect_err("selected map value must fail");
    assert_eq!(namespaces(error), [r#"Labels.values["source"]"#]);
}

#[tokio::test]
async fn filter_uses_positive_relative_namespace_semantics() {
    let value = NestedUser {
        name: String::new(),
        profile: Profile {
            email: String::new(),
            display_name: String::new(),
        },
    };
    let visited = RefCell::new(Vec::new());

    let error = Validator::new()
        .filter(&value, |namespace| {
            visited.borrow_mut().push(namespace.as_str().to_owned());
            matches!(namespace.as_str(), "profile" | "profile.email")
        })
        .await
        .expect_err("retained email must fail");

    assert_eq!(namespaces(error), ["NestedUser.profile.email"]);
    assert!(visited.borrow().contains(&"profile".to_owned()));
    assert!(visited.borrow().contains(&"profile.email".to_owned()));
}

#[tokio::test]
async fn selective_validation_filters_struct_level_errors() {
    let value = CheckedPair {
        left: String::new(),
        right: String::new(),
    };

    let partial = Validator::new()
        .partial(&value, ["right"])
        .await
        .expect_err("selected struct error must remain");
    assert_eq!(namespaces(partial), ["CheckedPair.right"]);

    let except = Validator::new()
        .except(&value, ["right"])
        .await
        .expect_err("non-excluded struct error must remain");
    assert_eq!(namespaces(except), ["CheckedPair.left"]);
}

#[tokio::test]
async fn unselected_field_does_not_preflight_runtime_alias() {
    let value = RuntimeAlias {
        hidden: String::new(),
        visible: "visible".to_owned(),
    };

    Validator::new()
        .partial(&value, ["visible"])
        .await
        .expect("unselected alias must not be resolved");
    assert!(Validator::new().validate(&value).await.is_err());
}

#[tokio::test]
async fn empty_selectors_have_stable_meaning() {
    let value = FlatUser {
        name: String::new(),
        email: String::new(),
    };

    Validator::new()
        .partial(&value, std::iter::empty::<&str>())
        .await
        .expect("empty partial selector validates nothing");
    assert!(
        Validator::new()
            .except(&value, std::iter::empty::<&str>())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn unknown_selectors_return_framework_errors() {
    let value = FlatUser {
        name: String::new(),
        email: String::new(),
    };

    assert!(matches!(
        Validator::new().partial(&value, ["emali"]).await,
        Err(Error::UnknownField { field }) if field == "emali"
    ));
    assert!(matches!(
        Validator::new().except(&value, ["profile.email"]).await,
        Err(Error::UnknownField { field }) if field == "profile.email"
    ));
}

#[tokio::test]
async fn filter_rejecting_a_parent_skips_its_complete_subtree() {
    let value = CheckedNestedPath {
        profile: Profile {
            email: String::new(),
            display_name: String::new(),
        },
    };

    Validator::new()
        .filter(&value, |namespace| namespace.as_str() == "profile.email")
        .await
        .expect("a rejected parent must suppress descendant field and struct errors");
}
