use mircap::ModuleImage;
use mirc0::compile;

fn check_fixture(text: &str) {
    let image = ModuleImage::from_text(text).expect("load fixture");
    let c_code = compile(&image, "main").expect("compile to C");
    assert!(!c_code.is_empty());
    assert!(c_code.contains("int main("));
    assert!(c_code.contains("mir_fn_"));
}

#[test]
fn emits_const_return() {
    check_fixture(include_str!("../../mircap/tests/fixtures/valid_const_return.mircap.txt"));
}

#[test]
fn emits_arithmetic() {
    check_fixture(include_str!("../../mircap/tests/fixtures/valid_arithmetic.mircap.txt"));
}

#[test]
fn emits_branch() {
    check_fixture(include_str!("../../mircap/tests/fixtures/valid_branch.mircap.txt"));
}

#[test]
fn emits_direct_call() {
    check_fixture(include_str!("../../mircap/tests/fixtures/valid_direct_call.mircap.txt"));
}

#[test]
fn emits_alloc_store_load_i32() {
    check_fixture(include_str!("../../mircap/tests/fixtures/valid_alloc_store_load_i32.mircap.txt"));
}

#[test]
fn emits_alloc_store_load_u32() {
    check_fixture(include_str!("../../mircap/tests/fixtures/valid_alloc_store_load_u32.mircap.txt"));
}
