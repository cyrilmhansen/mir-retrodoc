use mircap::ModuleImage;

fn load_fixture(name: &str) -> ModuleImage {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(path).expect("read fixture");
    ModuleImage::from_bytes(&bytes).expect("load fixture")
}

#[test]
fn valid_const_return_loads() {
    let image = load_fixture("valid_const_return.mircap.txt");
    let report = image.validate().expect("valid fixture");
    assert_eq!(report.function_count, 1);
}

#[test]
fn valid_arithmetic_loads() {
    load_fixture("valid_arithmetic.mircap.txt").validate().expect("valid fixture");
}

#[test]
fn valid_branch_loads() {
    load_fixture("valid_branch.mircap.txt").validate().expect("valid fixture");
}

#[test]
fn valid_direct_call_loads() {
    load_fixture("valid_direct_call.mircap.txt").validate().expect("valid fixture");
}

#[test]
fn valid_alloc_store_load_i32_loads() {
    load_fixture("valid_alloc_store_load_i32.mircap.txt").validate().expect("valid fixture");
}

#[test]
fn valid_alloc_store_load_u32_loads() {
    load_fixture("valid_alloc_store_load_u32.mircap.txt").validate().expect("valid fixture");
}

#[test]
fn valid_data_segment_loads() {
    load_fixture("valid_data_segment.mircap.txt").validate().expect("valid fixture");
}
