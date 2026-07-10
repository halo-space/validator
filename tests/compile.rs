#[test]
fn public_collection_contracts_compile() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/collection_rules.rs");
    tests.pass("tests/ui/dive_nested.rs");
}
