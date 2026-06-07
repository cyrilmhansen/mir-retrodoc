use std::path::Path;

fn run_mirtool(args: &[&str]) -> std::process::Output {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run")
       .arg("--bin")
       .arg("mirtool")
       .arg("--");
    for &arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("failed to execute cargo run")
}

fn fixture_path(name: &str) -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../mircap/tests/fixtures")
        .join(name)
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_validate_valid_const_return() {
    let path = fixture_path("valid_const_return.mircap.txt");
    let output = run_mirtool(&["validate", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "OK");
}

#[test]
fn test_encode_and_validate_binary() {
    let text_path = fixture_path("valid_const_return.mircap.txt");
    let temp_bin = "temp_smoke_test.mircap";
    
    // Encode text to binary
    let output = run_mirtool(&["encode", &text_path, temp_bin, "--force"]);
    assert!(output.status.success());
    
    // Validate binary
    let output2 = run_mirtool(&["validate", temp_bin]);
    assert!(output2.status.success());
    let stdout = String::from_utf8_lossy(&output2.stdout);
    assert_eq!(stdout.trim(), "OK");
    
    let _ = std::fs::remove_file(temp_bin);
}

#[test]
fn test_decode_binary() {
    let text_path = fixture_path("valid_const_return.mircap.txt");
    let temp_bin = "temp_smoke_decode.mircap";
    
    // Encode text to binary
    let output = run_mirtool(&["encode", &text_path, temp_bin, "--force"]);
    assert!(output.status.success());
    
    // Decode binary
    let output2 = run_mirtool(&["decode", temp_bin]);
    assert!(output2.status.success());
    let stdout = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout.contains("mircap"));
    assert!(stdout.contains("const_return"));
    assert!(stdout.contains("const_i32"));
    
    let _ = std::fs::remove_file(temp_bin);
}

#[test]
fn test_run_valid_sieve_32_u32() {
    let path = fixture_path("valid_sieve_32_u32.mircap.txt");
    let output = run_mirtool(&["run", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Result: u32 11"));
}

#[test]
fn test_compile_c_valid_sieve_32_u32() {
    let path = fixture_path("valid_sieve_32_u32.mircap.txt");
    let temp_c = "temp_smoke_sieve.c";
    
    let output = run_mirtool(&["compile-c", &path, temp_c]);
    assert!(output.status.success());
    
    let content = std::fs::read_to_string(temp_c).expect("read temp c");
    assert!(content.contains("mir_data_addr"));
    assert!(content.contains("mir_load_u32"));
    
    let _ = std::fs::remove_file(temp_c);
}

#[test]
fn test_diff_valid_sieve_32_u32() {
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_ok() {
        let path = fixture_path("valid_sieve_32_u32.mircap.txt");
        let output = run_mirtool(&["diff", &path]);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "PASS");
    }
}

#[test]
fn test_run_trap_load_oob() {
    let path = fixture_path("trap_load_oob.mircap.txt");
    let output = run_mirtool(&["run", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Trap: 13 OutOfBoundsLoad"));
}
