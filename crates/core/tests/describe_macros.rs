#[test]
fn describe_macros_compile_tests() {
    let t = trybuild::TestCases::new();

    t.pass("tests/compile/pass/*.rs");
    t.compile_fail("tests/compile/fail/*.rs");
}
