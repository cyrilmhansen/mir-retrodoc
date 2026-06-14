use mircap::ModuleImage;
use mirplan::{build_compile_plan, lower_compile_plan};
use mirsem::{ExecutionProfile, Runner, Value};
use mirspace::ProgramSpace;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn load_fixture(text: &str) -> ModuleImage {
    ModuleImage::from_text(text).expect("load fixture")
}

fn lowered_from_image(image: &ModuleImage) -> mirplan::LoweredProgram {
    let space = ProgramSpace::from_module_image(image).expect("space");
    let plan = build_compile_plan(&space);
    lower_compile_plan(&plan)
}

fn expected_result_line(image: &ModuleImage) -> String {
    let mut runner = Runner::new(image.clone(), ExecutionProfile::default()).expect("runner");
    let result = runner
        .run_entry_by_name("main", &[])
        .expect("fixture should return normally");
    match result.values.first() {
        None | Some(Value::Void) => "Result: void".to_string(),
        Some(Value::I32(value)) => format!("Result: i32 {value}"),
        Some(Value::U32(value)) => format!("Result: u32 {value}"),
        Some(Value::Addr32(value)) => format!("Result: addr32 {value}"),
    }
}

fn compile_and_run_c(test_name: &str, c_code: &str) -> Option<String> {
    let cc_check = Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        return None;
    }

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = dir.join(format!("temp_lowered_{test_name}.c"));
    let bin_path = dir.join(format!("temp_lowered_{test_name}"));
    fs::write(&c_path, c_code).expect("write lowered C");

    let compile_output = Command::new("cc")
        .arg("-O0")
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&bin_path)
        .arg(&c_path)
        .output()
        .expect("run cc");
    assert!(
        compile_output.status.success(),
        "lowered C compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let output = Command::new(&bin_path).output().expect("run lowered C");
    let _ = fs::remove_file(&c_path);
    let _ = fs::remove_file(&bin_path);

    assert!(
        output.status.success(),
        "lowered C execution failed: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.starts_with("Result: "))
            .expect("result line")
            .to_string(),
    )
}

fn check_lowered_fixture(test_name: &str, text: &str) {
    let image = load_fixture(text);
    let lowered = lowered_from_image(&image);

    let image_c = mirc0::compile(&image, "main").expect("compile from ModuleImage");
    let lowered_c = mirc0::compile_lowered(&lowered, "main").expect("compile from LoweredProgram");

    assert!(lowered_c.contains("int main("));
    assert!(lowered_c.contains("mir_fn_"));
    assert_eq!(
        image_c.contains("init_data_segments"),
        lowered_c.contains("init_data_segments")
    );

    if let Some(result_line) = compile_and_run_c(test_name, &lowered_c) {
        assert_eq!(result_line, expected_result_line(&image));
    }
}

#[test]
fn compile_lowered_const_return() {
    check_lowered_fixture(
        "const_return",
        include_str!("../../mircap/tests/fixtures/valid_const_return.mircap.txt"),
    );
}

#[test]
fn compile_lowered_branch() {
    check_lowered_fixture(
        "branch",
        include_str!("../../mircap/tests/fixtures/valid_branch.mircap.txt"),
    );
}

#[test]
fn compile_lowered_direct_call() {
    check_lowered_fixture(
        "direct_call",
        include_str!("../../mircap/tests/fixtures/valid_direct_call.mircap.txt"),
    );
}

#[test]
fn compile_lowered_data_segment_load() {
    check_lowered_fixture(
        "data_segment_load",
        include_str!("../../mircap/tests/fixtures/valid_data_segment_load.mircap.txt"),
    );
}
