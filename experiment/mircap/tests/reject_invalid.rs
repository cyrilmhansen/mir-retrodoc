use mircap::ModuleImage;

fn reject_fixture(name: &str) {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(path).expect("read fixture");
    let image = ModuleImage::from_bytes(&bytes).expect("load fixture");
    assert!(image.validate().is_err(), "{name} should be rejected");
}

#[test]
fn rejects_duplicate_function_id() {
    reject_fixture("invalid_duplicate_function_id.mircap.txt");
}

#[test]
fn rejects_missing_block() {
    reject_fixture("invalid_missing_block.mircap.txt");
}

#[test]
fn rejects_wrong_call_arity() {
    reject_fixture("invalid_wrong_call_arity.mircap.txt");
}

#[test]
fn rejects_return_type_mismatch() {
    reject_fixture("invalid_return_type_mismatch.mircap.txt");
}

#[test]
fn rejects_unsupported_i64() {
    reject_fixture("invalid_unsupported_i64.mircap.txt");
}

#[test]
fn rejects_instruction_after_terminator() {
    reject_fixture("invalid_instruction_after_terminator.mircap.txt");
}

#[test]
fn rejects_block_without_terminator() {
    reject_fixture("invalid_block_without_terminator.mircap.txt");
}

#[test]
fn rejects_load_non_addr32() {
    reject_fixture("invalid_load_non_addr32.mircap.txt");
}

#[test]
fn rejects_store_wrong_value_type() {
    reject_fixture("invalid_store_wrong_value_type.mircap.txt");
}

#[test]
fn rejects_alloc_wrong_result_type() {
    reject_fixture("invalid_alloc_wrong_result_type.mircap.txt");
}

#[test]
fn rejects_malformed_data_segment() {
    reject_fixture("invalid_malformed_data_segment.mircap.txt");
}

#[test]
fn rejects_unsupported_i64_memory() {
    reject_fixture("invalid_unsupported_i64_memory.mircap.txt");
}

#[test]
fn rejects_addr_add_wrong_offset_type() {
    reject_fixture("invalid_addr_add_wrong_offset_type.mircap.txt");
}

#[test]
fn rejects_addr_add_wrong_base_type() {
    reject_fixture("invalid_addr_add_wrong_base_type.mircap.txt");
}

#[test]
fn rejects_addr_add_addr32_offset() {
    reject_fixture("invalid_addr_add_addr32_offset.mircap.txt");
}
