#[test]
fn macro_compile_pass_cases() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/trybuild/pass_*.rs");
    tests.compile_fail("tests/trybuild/fail_*.rs");
}
