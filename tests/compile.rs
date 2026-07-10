#[test]
fn public_collection_contracts_compile() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/collection_rules.rs");
    tests.pass("tests/ui/dive_nested.rs");
    tests.compile_fail("tests/ui/unique_field_invalid_container.rs");
    tests.compile_fail("tests/ui/unique_field_missing.rs");
    tests.compile_fail("tests/ui/unique_field_path.rs");
    tests.compile_fail("tests/ui/unique_field_syntax.rs");
}
