use std::path::Path;

fn run_mirtool(args: &[&str]) -> std::process::Output {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run").arg("--bin").arg("mirtool").arg("--");
    for &arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("failed to execute cargo run")
}

fn fixture_path(name: &str) -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../mircap/tests/fixtures")
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
fn test_dump_binary() {
    let text_path = fixture_path("valid_const_return.mircap.txt");
    let temp_bin = "temp_smoke_dump.mircap";

    // Encode text to binary
    let output = run_mirtool(&["encode", &text_path, temp_bin, "--force"]);
    assert!(output.status.success());

    // Dump binary
    let output2 = run_mirtool(&["dump", temp_bin]);
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
fn test_run_valid_float_arithmetic() {
    let path = fixture_path("valid_float_arithmetic.mircap.txt");
    let output = run_mirtool(&["run", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Result: f32 -16 bits=0xc1800000"));
    assert!(stdout.contains("Result: f64 -16 bits=0xc030000000000000"));
}

#[test]
fn test_run_trace_summary() {
    let path = fixture_path("valid_sieve_32_u32.mircap.txt");
    let output = run_mirtool(&["run", &path, "--trace"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Result: u32 11"));
    assert!(stdout.contains("Trace Summary"));
    assert!(stdout.contains("Executed Instructions:"));
    assert!(stdout.contains("Maximum Call Depth:"));
    assert!(stdout.contains("Allocations:"));
    assert!(stdout.contains("Allocated Bytes:"));
}

#[test]
fn test_plan_valid_branch() {
    let path = fixture_path("valid_branch.mircap.txt");
    let output = run_mirtool(&["plan", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("module branch"));
    assert!(stdout.contains("fn f0#1 main"));
    assert!(stdout.contains("i1#2 branch_if"));
    assert!(stdout.contains("successors: true:b1#2, false:b2#3"));
}

#[test]
fn test_plan_binary_input() {
    let text_path = fixture_path("valid_branch.mircap.txt");
    let temp_bin = "temp_smoke_plan.mircap";

    let output = run_mirtool(&["encode", &text_path, temp_bin, "--force"]);
    assert!(output.status.success());

    let output = run_mirtool(&["plan", temp_bin]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("module branch"));
    assert!(stdout.contains("successors: true:b1#2, false:b2#3"));

    let _ = std::fs::remove_file(temp_bin);
}

#[test]
fn test_lower_valid_branch() {
    let path = fixture_path("valid_branch.mircap.txt");
    let output = run_mirtool(&["lower", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lowered module branch"));
    assert!(stdout.contains("fn f0#1 main"));
    assert!(stdout.contains("branch branch_if"));
    assert!(stdout.contains("targets=[true:b1#2, false:b2#3]"));
    assert!(stdout.contains("successors: true:b1#2, false:b2#3"));
}

#[test]
fn test_lower_binary_input() {
    let text_path = fixture_path("valid_direct_call.mircap.txt");
    let temp_bin = "temp_smoke_lower.mircap";

    let output = run_mirtool(&["encode", &text_path, temp_bin, "--force"]);
    assert!(output.status.success());

    let output = run_mirtool(&["lower", temp_bin]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lowered module direct_call"));
    assert!(stdout.contains("call call writes=[v1#1:i32] reads=[v0#0:i32]"));
    assert!(stdout.contains("callee=f1#2 callee"));

    let _ = std::fs::remove_file(temp_bin);
}

#[test]
fn test_analyze_valid_arithmetic_u32() {
    let path = fixture_path("valid_arithmetic_u32.mircap.txt");
    let output = run_mirtool(&["analyze", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("analysis module arithmetic_u32"));
    assert!(stdout.contains("pure_candidate: true"));
    assert!(stdout.contains("guaranteed_terminates_trivially: true"));
    assert!(stdout.contains("calls: -"));
}

#[test]
fn test_analyze_valid_memory_effects() {
    let path = fixture_path("valid_alloc_store_load_u32.mircap.txt");
    let output = run_mirtool(&["analyze", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("allocates: true"));
    assert!(stdout.contains("reads_memory: true"));
    assert!(stdout.contains("writes_memory: true"));
    assert!(stdout.contains("may_trap: true"));
    assert!(stdout.contains("pure_candidate: false"));
}

#[test]
fn test_analyze_valid_direct_call() {
    let path = fixture_path("valid_direct_call.mircap.txt");
    let output = run_mirtool(&["analyze", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("calls: f1#2 callee"));
    assert!(stdout.contains("pure_candidate: false"));
}

#[test]
fn test_analyze_valid_loop() {
    let path = fixture_path("valid_loop.mircap.txt");
    let output = run_mirtool(&["analyze", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("acyclic_cfg: false"));
    assert!(stdout.contains("guaranteed_terminates_trivially: false"));
}

#[test]
fn test_trace_check_valid_memory_effects() {
    let path = fixture_path("valid_alloc_store_load_u32.mircap.txt");
    let output = run_mirtool(&["trace-check", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("trace-check module alloc_store_load_u32"));
    assert!(stdout.contains("outcome: returned"));
    assert!(stdout.contains("allocates: static=true observed=1 status=observed"));
    assert!(stdout.contains("reads_memory: static=true observed=1 status=observed"));
    assert!(stdout.contains("writes_memory: static=true observed=1 status=observed"));
    assert!(stdout.contains("may_trap: static=true observed=0 status=conservative"));
}

#[test]
fn test_trace_check_valid_arithmetic_absent_effects() {
    let path = fixture_path("valid_arithmetic_u32.mircap.txt");
    let output = run_mirtool(&["trace-check", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("trace-check module arithmetic_u32"));
    assert!(stdout.contains("allocates: static=false observed=0 status=proven-absent"));
    assert!(stdout.contains("reads_memory: static=false observed=0 status=proven-absent"));
    assert!(stdout.contains("writes_memory: static=false observed=0 status=proven-absent"));
    assert!(stdout.contains("may_trap: static=false observed=0 status=proven-absent"));
}

#[test]
fn test_trace_check_trap_fixture() {
    let path = fixture_path("trap_load_oob.mircap.txt");
    let output = run_mirtool(&["trace-check", &path]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("trace-check module load_oob"));
    assert!(stdout.contains("outcome: trapped 13 OutOfBoundsLoad"));
    assert!(stdout.contains("may_trap: static=true observed=1 status=observed"));
}

#[test]
fn test_bench_load_text() {
    let path = fixture_path("valid_data_segment_load.mircap.txt");
    let output = run_mirtool(&["bench-load", &path, "--iterations", "3"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format: text"));
    assert!(stdout.contains("iterations: 3"));
    assert!(stdout.contains("avg_ns:"));
    assert!(stdout.contains("checksum:"));
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
fn test_diff_valid_i64_ops() {
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_ok() {
        let path = fixture_path("valid_i64_ops.mircap.txt");
        let output = run_mirtool(&["diff", &path]);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "PASS");
    }
}

#[test]
fn test_diff_valid_float_arithmetic() {
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_ok() {
        let path = fixture_path("valid_float_arithmetic.mircap.txt");
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

#[test]
fn test_binary_sieve_32_u32_flow() {
    let text_path = fixture_path("valid_sieve_32_u32.mircap.txt");
    let temp_bin = "temp_smoke_sieve.mircap";
    let temp_c = "temp_smoke_sieve_bin.c";
    let temp_exec = "temp_smoke_sieve_bin_exe";

    // 1. Encode
    let output = run_mirtool(&["encode", &text_path, temp_bin, "--force"]);
    assert!(output.status.success());

    // 2. Validate
    let output = run_mirtool(&["validate", temp_bin]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "OK");

    // 3. Run
    let output = run_mirtool(&["run", temp_bin]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Result: u32 11"));

    // 4. Compile C
    let output = run_mirtool(&["compile-c", temp_bin, temp_c]);
    assert!(output.status.success());

    // 5. Build with CC and Run
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_ok() {
        let compile_res = std::process::Command::new("cc")
            .arg("-std=c11")
            .arg("-Wall")
            .arg("-Wextra")
            .arg("-Werror")
            .arg("-O0")
            .arg("-o")
            .arg(temp_exec)
            .arg(temp_c)
            .output();
        assert!(compile_res.is_ok());
        let compile_output = compile_res.unwrap();
        assert!(
            compile_output.status.success(),
            "C compilation failed:\n{}",
            String::from_utf8_lossy(&compile_output.stderr)
        );

        let exec_res = std::process::Command::new(format!("./{}", temp_exec)).output();
        assert!(exec_res.is_ok());
        let exec_output = exec_res.unwrap();
        assert!(exec_output.status.success());
        assert!(String::from_utf8_lossy(&exec_output.stdout).contains("Result: u32 11"));

        let _ = std::fs::remove_file(temp_exec);
    }

    // 6. Diff
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_ok() {
        let output = run_mirtool(&["diff", temp_bin]);
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }

    let _ = std::fs::remove_file(temp_bin);
    let _ = std::fs::remove_file(temp_c);
}

#[test]
fn test_binary_trap_load_oob_flow() {
    let text_path = fixture_path("trap_load_oob.mircap.txt");
    let temp_bin = "temp_smoke_trap.mircap";

    // 1. Encode
    let output = run_mirtool(&["encode", &text_path, temp_bin, "--force"]);
    assert!(output.status.success());

    // 2. Validate
    let output = run_mirtool(&["validate", temp_bin]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "OK");

    // 3. Run binary trap
    let output = run_mirtool(&["run", temp_bin]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Trap: 13 OutOfBoundsLoad"));

    // 4. Diff binary trap
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_ok() {
        let output = run_mirtool(&["diff", temp_bin]);
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }

    let _ = std::fs::remove_file(temp_bin);
}

#[test]
fn test_diff_upstream_valid_const_return() {
    let m2b_check =
        std::path::Path::new("/home/john/project/mir-preservation/git/mir-restored/m2b").exists();
    if m2b_check {
        let path = fixture_path("valid_const_return.mircap.txt");
        let output = run_mirtool(&["diff-upstream", &path]);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "PASS");
    }
}

#[test]
fn test_diff_upstream_valid_sieve_32_u32() {
    let m2b_check =
        std::path::Path::new("/home/john/project/mir-preservation/git/mir-restored/m2b").exists();
    if m2b_check {
        let path = fixture_path("valid_sieve_32_u32.mircap.txt");
        let output = run_mirtool(&["diff-upstream", &path]);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "PASS");
    }
}

#[test]
fn test_diff_upstream_trap_load_oob() {
    let m2b_check =
        std::path::Path::new("/home/john/project/mir-preservation/git/mir-restored/m2b").exists();
    if m2b_check {
        let path = fixture_path("trap_load_oob.mircap.txt");
        let output = run_mirtool(&["diff-upstream", &path]);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "PASS");
    }
}

#[test]
fn test_diff_upstream_valid_i64_ops() {
    let m2b_check =
        std::path::Path::new("/home/john/project/mir-preservation/git/mir-restored/m2b").exists();
    if m2b_check {
        let path = fixture_path("valid_i64_ops.mircap.txt");
        let output = run_mirtool(&["diff-upstream", &path]);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "PASS");
    }
}

#[test]
fn test_compile_rv32i_valid_const_return() {
    let path = fixture_path("valid_const_return.mircap.txt");
    let temp_asm = "temp_smoke_compile_rv32i.s";
    let output = run_mirtool(&["compile-rv32i", &path, temp_asm]);
    assert!(output.status.success());

    let content = std::fs::read_to_string(temp_asm).expect("read temp asm");
    assert!(content.contains(".attribute arch"));
    assert!(content.contains("mir_fn_1"));

    let _ = std::fs::remove_file(temp_asm);
}

#[test]
fn test_diff_all() {
    let output = run_mirtool(&["diff-all"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Summary:"));
    assert!(stdout.contains("Passed"));
}
