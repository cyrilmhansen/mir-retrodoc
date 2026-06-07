use mircap::ModuleImage;

fn check_fixture_roundtrip(name: &str) {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    let text = std::fs::read_to_string(path).expect("read fixture");
    
    // 1. Text -> ModuleImage
    let original = ModuleImage::from_text(&text).expect("parse original ModuleImage");
    
    // 2. Validate original
    let orig_validation = original.validate();
    assert!(orig_validation.is_ok(), "Original fixture {} must be valid: {:?}", name, orig_validation);
    let orig_report = orig_validation.unwrap();
    
    // 3. ModuleImage -> capnp bytes
    let capnp_bytes = original.to_capnp_bytes();
    assert!(!capnp_bytes.is_empty(), "Serialized capnp bytes must not be empty");
    
    // 4. capnp bytes -> ModuleImage
    let decoded = ModuleImage::from_capnp_bytes(&capnp_bytes).expect("deserialize capnp bytes");
    
    // 5. Compare logical equality
    assert_eq!(original, decoded, "Decoded ModuleImage must be logically equal to original for fixture {}", name);
    
    // 6. Validate decoded
    let dec_validation = decoded.validate();
    assert!(dec_validation.is_ok(), "Decoded fixture {} must be valid: {:?}", name, dec_validation);
    let dec_report = dec_validation.unwrap();
    
    // 7. Verify validation results match
    assert_eq!(orig_report.function_count, dec_report.function_count);
}

#[test]
fn roundtrip_valid_const_return() {
    check_fixture_roundtrip("valid_const_return.mircap.txt");
}

#[test]
fn roundtrip_valid_arithmetic_u32() {
    check_fixture_roundtrip("valid_arithmetic_u32.mircap.txt");
}

#[test]
fn roundtrip_valid_load_store_u8() {
    check_fixture_roundtrip("valid_load_store_u8.mircap.txt");
}

#[test]
fn roundtrip_valid_data_segment_load() {
    check_fixture_roundtrip("valid_data_segment_load.mircap.txt");
}

#[test]
fn roundtrip_valid_sieve_32_u32() {
    check_fixture_roundtrip("valid_sieve_32_u32.mircap.txt");
}

#[test]
fn roundtrip_trap_load_oob() {
    check_fixture_roundtrip("trap_load_oob.mircap.txt");
}
