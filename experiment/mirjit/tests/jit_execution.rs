use mircap::ModuleImage;
use mirsem::profile::ExecutionProfile;
use mirsem::value::Value;
use mirsem::trap::ExecutionTrap;
use mirjit::{JitContext, ThunkTarget, JitError};
use mirplan::{build_compile_plan, lower_compile_plan, Backend};
use mirc0::C11Backend;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::error::Error;

const CONST_RETURN_FIXTURE: &str = include_str!("../../mircap/tests/fixtures/valid_const_return.mircap.txt");
const TRAP_FIXTURE: &str = include_str!("../../mircap/tests/fixtures/trap_load_oob.mircap.txt");

fn compile_function_to_bin(image: &ModuleImage, test_name: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let space = mirspace::ProgramSpace::from_module_image(image)?;
    let plan = build_compile_plan(&space);
    let lowered = lower_compile_plan(&plan);

    let backend = C11Backend::new("main");
    let c_code = backend.compile(&lowered)?;

    let cc_check = Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        return Err("C compiler 'cc' is unavailable".into());
    }

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = fs::create_dir_all(&dir);
    let c_path = dir.join(format!("temp_jit_{}.c", test_name));
    let bin_path = dir.join(format!("temp_jit_{}", test_name));

    fs::write(&c_path, c_code)?;

    let mut compile_cmd = Command::new("cc");
    compile_cmd.arg("-O0")
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&bin_path)
        .arg(&c_path);
    
    let compile_output = compile_cmd.output()?;
    let _ = fs::remove_file(&c_path);

    if !compile_output.status.success() {
        return Err(format!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&compile_output.stderr)
        ).into());
    }

    Ok(bin_path.to_string_lossy().to_string())
}

#[test]
fn test_interpreter_mode() {
    let image = ModuleImage::from_text(CONST_RETURN_FIXTURE).unwrap();
    let context = JitContext::new(image, ExecutionProfile::default());

    let res = context.call_by_name("main", &[]).unwrap();
    assert_eq!(res.values, vec![Value::I32(42)]);
}

#[test]
fn test_eager_compile_mode() {
    let cc_check = Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        return; // Skip if compiler unavailable
    }

    let image = ModuleImage::from_text(CONST_RETURN_FIXTURE).unwrap();
    let mut context = JitContext::new(image, ExecutionProfile::default());

    context.set_eager_compile(|img, _| {
        compile_function_to_bin(img, "eager_const_return")
    }).unwrap();

    let thunk = context.thunks.values().find(|t| t.name == "main").unwrap();
    assert!(matches!(thunk.target(), ThunkTarget::Compiled { .. }));

    let res = context.call_by_name("main", &[]).unwrap();
    assert_eq!(res.values, vec![Value::I32(42)]);

    // Clean up compiled binary
    if let ThunkTarget::Compiled { binary_path } = thunk.target() {
        let _ = fs::remove_file(binary_path);
    }
}

#[test]
fn test_lazy_compile_mode() {
    let cc_check = Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        return;
    }

    let image = ModuleImage::from_text(CONST_RETURN_FIXTURE).unwrap();
    let mut context = JitContext::new(image, ExecutionProfile::default());

    let compile_counter = Arc::new(Mutex::new(0));
    let counter_clone = compile_counter.clone();

    let compile_hook = Arc::new(move |img: &ModuleImage, _| {
        *counter_clone.lock().unwrap() += 1;
        compile_function_to_bin(img, "lazy_const_return")
    });

    context.set_lazy_compile(compile_hook);

    let thunk = context.thunks.values().find(|t| t.name == "main").unwrap();
    assert!(matches!(thunk.target(), ThunkTarget::LazyCompile { .. }));
    assert_eq!(*compile_counter.lock().unwrap(), 0);

    // First call triggers compilation
    let res = context.call_by_name("main", &[]).unwrap();
    assert_eq!(res.values, vec![Value::I32(42)]);
    assert_eq!(*compile_counter.lock().unwrap(), 1);
    assert!(matches!(thunk.target(), ThunkTarget::Compiled { .. }));

    // Second call bypasses compilation (uses cached Compiled target)
    let res2 = context.call_by_name("main", &[]).unwrap();
    assert_eq!(res2.values, vec![Value::I32(42)]);
    assert_eq!(*compile_counter.lock().unwrap(), 1);

    // Clean up compiled binary
    if let ThunkTarget::Compiled { binary_path } = thunk.target() {
        let _ = fs::remove_file(binary_path);
    }
}

#[test]
fn test_interpreter_trap_mode() {
    let image = ModuleImage::from_text(TRAP_FIXTURE).unwrap();
    let context = JitContext::new(image, ExecutionProfile::default());

    let res = context.call_by_name("main", &[]);
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), JitError::InterpreterRun(..)));
}

#[test]
fn test_compiled_trap_mode() {
    let cc_check = Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        return;
    }

    let image = ModuleImage::from_text(TRAP_FIXTURE).unwrap();
    let mut context = JitContext::new(image, ExecutionProfile::default());

    context.set_eager_compile(|img, _| {
        compile_function_to_bin(img, "trap_const_return")
    }).unwrap();

    let thunk = context.thunks.values().find(|t| t.name == "main").unwrap();
    let res = context.call_by_name("main", &[]);
    assert!(res.is_err());
    
    match res.unwrap_err() {
        JitError::Trap(ExecutionTrap::OutOfBoundsLoad { .. }) => {}
        e => panic!("Expected OutOfBoundsLoad trap, got: {:?}", e),
    }

    // Clean up compiled binary
    if let ThunkTarget::Compiled { binary_path } = thunk.target() {
        let _ = fs::remove_file(binary_path);
    }
}
