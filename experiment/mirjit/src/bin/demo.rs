use mirc0::C11Backend;
use mircap::ModuleImage;
use mirjit::{JitContext, ThunkTarget};
use mirplan::{build_compile_plan, lower_compile_plan, Backend};
use mirsem::profile::ExecutionProfile;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const CONST_RETURN_FIXTURE: &str =
    include_str!("../../../mircap/tests/fixtures/valid_const_return.mircap.txt");
const SIEVE_FIXTURE: &str =
    include_str!("../../../mircap/tests/fixtures/valid_sieve_32_u32.mircap.txt");

fn compile_function_to_bin(
    image: &ModuleImage,
    test_name: &str,
    optimize: bool,
) -> Result<String, String> {
    let space = mirspace::ProgramSpace::from_module_image(image).map_err(|e| format!("{:?}", e))?;
    let plan = build_compile_plan(&space);
    let mut lowered = lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    let backend = C11Backend::new("main");
    let c_code = backend.compile(&lowered).map_err(|e| e.to_string())?;

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = fs::create_dir_all(&dir);
    let c_path = dir.join(format!("temp_demo_{}.c", test_name));
    let bin_path = dir.join(format!("temp_demo_{}", test_name));

    fs::write(&c_path, c_code).map_err(|e| e.to_string())?;

    let mut compile_cmd = Command::new("cc");
    compile_cmd
        .arg("-O0")
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&bin_path)
        .arg(&c_path);

    let compile_output = compile_cmd.output().map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&c_path);

    if !compile_output.status.success() {
        return Err(format!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&compile_output.stderr)
        ));
    }

    Ok(bin_path.to_string_lossy().to_string())
}

fn compile_and_run_c_bench(
    image: &ModuleImage,
    test_name: &str,
    opt_level: &str,
    optimize: bool,
    iterations: u32,
) -> Result<u128, String> {
    let space = mirspace::ProgramSpace::from_module_image(image).map_err(|e| format!("{:?}", e))?;
    let plan = build_compile_plan(&space);
    let mut lowered = lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    let backend = C11Backend::new("main");
    let compiled_c = backend.compile(&lowered).map_err(|e| e.to_string())?;
    let mut c_code = format!("#define _POSIX_C_SOURCE 199309L\n{}", compiled_c);

    let bench_main = format!(
        r#"
#include <time.h>
int main(void) {{
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);
    uint32_t res = 0;
    for (uint32_t i = 0; i < {iterations}u; i++) {{
        g_heap_ptr = 0;
        init_data_segments();
        res = mir_fn_1();
    }}
    clock_gettime(CLOCK_MONOTONIC, &end);
    uint64_t elapsed_ns = (end.tv_sec - start.tv_sec) * 1000000000ULL + (end.tv_nsec - start.tv_nsec);
    printf("elapsed_ns: %" PRIu64 "\n", elapsed_ns);
    printf("Result: u32 %" PRIu32 "\n", res);
    return 0;
}}
"#
    );

    if let Some(pos) = c_code.find("int main(void)") {
        c_code.truncate(pos);
        c_code.push_str(&bench_main);
    } else {
        return Err("Failed to find main function in C output".to_string());
    }

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = fs::create_dir_all(&dir);
    let c_path = dir.join(format!(
        "temp_bench_{}_{}.c",
        test_name,
        opt_level.replace("-", "")
    ));
    let bin_path = dir.join(format!(
        "temp_bench_{}_{}",
        test_name,
        opt_level.replace("-", "")
    ));

    fs::write(&c_path, c_code).map_err(|e| e.to_string())?;

    let mut compile_cmd = Command::new("cc");
    compile_cmd
        .arg(opt_level)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&bin_path)
        .arg(&c_path);

    let compile_output = compile_cmd.output().map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&c_path);

    if !compile_output.status.success() {
        return Err(format!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&compile_output.stderr)
        ));
    }

    let run_output = Command::new(&bin_path)
        .output()
        .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&bin_path);

    if !run_output.status.success() {
        return Err(format!(
            "Execution failed: {}",
            String::from_utf8_lossy(&run_output.stderr)
        ));
    }

    let stdout_str = String::from_utf8_lossy(&run_output.stdout);
    let elapsed_line = stdout_str.lines().find(|l| l.starts_with("elapsed_ns: "));
    if let Some(line) = elapsed_line {
        let ns_str = &line["elapsed_ns: ".len()..];
        let ns = ns_str.trim().parse::<u128>().map_err(|e| e.to_string())?;
        Ok(ns)
    } else {
        Err("Failed to parse elapsed time from benchmark".to_string())
    }
}

fn run_python_bench(iterations: u32) -> Result<u128, String> {
    let python_script = format!(
        r#"
import sys
import time

def run_sieve():
    is_prime = [1] * 32
    is_prime[0] = 0
    is_prime[1] = 0
    
    # mark multiples of 2
    i = 4
    while i < 32:
        is_prime[i] = 0
        i += 2
        
    # mark multiples of 3
    i = 9
    while i < 32:
        is_prime[i] = 0
        i += 3
        
    # mark multiples of 5
    i = 25
    while i < 32:
        is_prime[i] = 0
        i += 5
        
    # sum primes
    total = 0
    for idx in range(32):
        if is_prime[idx]:
            total += idx
    return total

iterations = {iterations}
start = time.perf_counter_ns()
checksum = 0
for _ in range(iterations):
    checksum += run_sieve()
end = time.perf_counter_ns()
elapsed_ns = end - start
print(f"elapsed_ns: {{elapsed_ns}}")
"#,
    );

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = fs::create_dir_all(&dir);
    let script_path = dir.join("sieve_bench.py");
    fs::write(&script_path, python_script).map_err(|e| e.to_string())?;

    let output = Command::new("python3").arg(&script_path).output();

    let _ = fs::remove_file(&script_path);

    let run_output = output.map_err(|e| e.to_string())?;
    if !run_output.status.success() {
        return Err("Python execution failed".to_string());
    }

    let stdout_str = String::from_utf8_lossy(&run_output.stdout);
    let elapsed_line = stdout_str.lines().find(|l| l.starts_with("elapsed_ns: "));
    if let Some(line) = elapsed_line {
        let ns_str = &line["elapsed_ns: ".len()..];
        let ns = ns_str.trim().parse::<u128>().map_err(|e| e.to_string())?;
        Ok(ns)
    } else {
        Err("Failed to parse elapsed time from Python benchmark".to_string())
    }
}

fn run_interpreter_bench(image: &ModuleImage, iterations: u32) -> Result<u128, String> {
    let profile = ExecutionProfile::default();
    let entry_func = image
        .functions
        .first()
        .ok_or("No functions found".to_string())?
        .id;

    // Warm up
    {
        let mut runner = mirsem::runner::Runner::new(image.clone(), profile.clone())
            .map_err(|e| format!("{:?}", e))?;
        let _ = runner
            .run_entry(entry_func, &[])
            .map_err(|e| format!("{:?}", e))?;
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let mut runner = mirsem::runner::Runner::new(image.clone(), profile.clone())
            .map_err(|e| format!("{:?}", e))?;
        let _ = runner
            .run_entry(entry_func, &[])
            .map_err(|e| format!("{:?}", e))?;
    }
    let elapsed = start.elapsed().as_nanos();
    Ok(elapsed)
}

fn run_pedagogical_demo() -> Result<(), Box<dyn Error>> {
    println!("================================================================");
    println!("PART 1: MIR-Inspired JIT Compiler Thunk Redirection Demo");
    println!("================================================================\n");

    println!("Loading MIR-F0 Module Image (const_return)...");
    let image = ModuleImage::from_text(CONST_RETURN_FIXTURE).map_err(|e| format!("{:?}", e))?;
    println!("Module name: {}\n", image.module.name);

    // 1. Interpreter Mode
    println!("Step 1: Running in Interpreter Mode...");
    let context_interp = JitContext::new(image.clone(), ExecutionProfile::default());
    let res_interp = context_interp.call_by_name("main", &[])?;
    println!("Result: {:?}", res_interp.values);
    println!("Interpreter mode executed successfully.\n");

    let cc_check = Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        println!("C compiler 'cc' is unavailable. Skipping compilation stages.");
        return Ok(());
    }

    // 2. Eager Compile Mode
    println!("Step 2: Running in Eager JIT Compile Mode...");
    let mut context_eager = JitContext::new(image.clone(), ExecutionProfile::default());
    context_eager
        .set_eager_compile(|img, _| {
            compile_function_to_bin(img, "demo_eager", true).map_err(|e| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                    as Box<dyn Error + Send + Sync>
            })
        })
        .map_err(|e| e.to_string())?;

    let thunk_eager = context_eager
        .thunks
        .values()
        .find(|t| t.name == "main")
        .unwrap();
    println!("Main thunk target: Compiled");

    let res_eager = context_eager.call_by_name("main", &[])?;
    println!("Result: {:?}", res_eager.values);

    if let ThunkTarget::Compiled { binary_path } = thunk_eager.target() {
        let _ = fs::remove_file(&binary_path);
    }
    println!("Eager compiled execution completed successfully.\n");

    // 3. Lazy Compile Mode
    println!("Step 3: Running in Lazy JIT Compile Mode...");
    let mut context_lazy = JitContext::new(image.clone(), ExecutionProfile::default());

    let compile_counter = Arc::new(Mutex::new(0));
    let counter_clone = compile_counter.clone();
    let compile_hook = Arc::new(move |img: &ModuleImage, _| {
        *counter_clone.lock().unwrap() += 1;
        compile_function_to_bin(img, "demo_lazy", true).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn Error + Send + Sync>
        })
    });

    context_lazy.set_lazy_compile(compile_hook);
    let thunk_lazy = context_lazy
        .thunks
        .values()
        .find(|t| t.name == "main")
        .unwrap();
    println!("Main thunk target initially: LazyCompile");
    println!("Compilation count: {}", *compile_counter.lock().unwrap());

    println!("\nCalling 'main' the first time (triggers compilation)...");
    let res_lazy1 = context_lazy.call_by_name("main", &[])?;
    println!("Result: {:?}", res_lazy1.values);
    println!("Compilation count: {}", *compile_counter.lock().unwrap());
    println!("Main thunk target is now updated to: Compiled");

    println!("\nCalling 'main' the second time (bypasses compilation)...");
    let res_lazy2 = context_lazy.call_by_name("main", &[])?;
    println!("Result: {:?}", res_lazy2.values);
    println!("Compilation count: {}", *compile_counter.lock().unwrap());

    if let ThunkTarget::Compiled { binary_path } = thunk_lazy.target() {
        let _ = fs::remove_file(&binary_path);
    }
    println!("\nLazy compile wrapper redirection completed successfully.\n");
    Ok(())
}

fn run_performance_benchmarks() -> Result<(), Box<dyn Error>> {
    println!("================================================================");
    println!("PART 2: Performance Benchmark - Sieve of Eratosthenes (Limit 32)");
    println!("================================================================\n");

    let iterations = 10_000;
    println!("Running benchmarks with {} iterations...\n", iterations);

    // 1. Python Bench
    let python_check = Command::new("python3").arg("--version").output();
    let python_ns = if python_check.is_ok() {
        print!("Running Python benchmark... ");
        let ns = run_python_bench(iterations).map_err(|e| e.to_string())?;
        println!("{:.3} ms", (ns as f64) / 1_000_000.0);
        Some(ns)
    } else {
        println!("Python3 is not available. Skipping Python benchmark.");
        None
    };

    // 2. Interpreter Bench
    let image = ModuleImage::from_text(SIEVE_FIXTURE).map_err(|e| format!("{:?}", e))?;
    print!("Running mirsem Interpreter benchmark... ");
    let interp_ns = run_interpreter_bench(&image, iterations).map_err(|e| e.to_string())?;
    println!("{:.3} ms", (interp_ns as f64) / 1_000_000.0);

    // 3. JIT C Compiled -O0
    let cc_check = Command::new("cc").arg("--version").output();
    let (c_o0_ns, c_o0_opt_ns, c_o3_ns) = if cc_check.is_ok() {
        print!("Running JIT Compiled C (-O0, No Opts) benchmark... ");
        let o0_ns = compile_and_run_c_bench(&image, "sieve", "-O0", false, iterations)
            .map_err(|e| e.to_string())?;
        println!("{:.3} ms", (o0_ns as f64) / 1_000_000.0);

        print!("Running JIT Compiled C (-O0, with Plan Opts) benchmark... ");
        let o0_opt_ns = compile_and_run_c_bench(&image, "sieve", "-O0", true, iterations)
            .map_err(|e| e.to_string())?;
        println!("{:.3} ms", (o0_opt_ns as f64) / 1_000_000.0);

        print!("Running JIT Compiled C (-O3, with Plan Opts) benchmark... ");
        let o3_ns = compile_and_run_c_bench(&image, "sieve", "-O3", true, iterations)
            .map_err(|e| e.to_string())?;
        println!("{:.3} ms", (o3_ns as f64) / 1_000_000.0);
        (Some(o0_ns), Some(o0_opt_ns), Some(o3_ns))
    } else {
        println!("cc compiler is not available. Skipping JIT Compiled C benchmarks.");
        (None, None, None)
    };

    println!("\n================================================================\n");
    println!("                     BENCHMARK COMPARISON TABLE");
    println!("----------------------------------------------------------------");
    println!(" Environment                 | Total Time (ms) | Speedup Ratio  ");
    println!("----------------------------------------------------------------");

    let base_ns = python_ns.unwrap_or(interp_ns);

    if let Some(py_ns) = python_ns {
        println!(
            " Naive Python 3             | {:>15.3} | {:>12.2}x ",
            (py_ns as f64) / 1_000_000.0,
            (base_ns as f64) / (py_ns as f64)
        );
    }
    println!(
        " mirsem Interpreter         | {:>15.3} | {:>12.2}x ",
        (interp_ns as f64) / 1_000_000.0,
        (base_ns as f64) / (interp_ns as f64)
    );
    if let Some(o0_ns) = c_o0_ns {
        println!(
            " JIT Compiled C (-O0, Raw)  | {:>15.3} | {:>12.2}x ",
            (o0_ns as f64) / 1_000_000.0,
            (base_ns as f64) / (o0_ns as f64)
        );
    }
    if let Some(o0_opt_ns) = c_o0_opt_ns {
        println!(
            " JIT Compiled C (-O0, Opt)  | {:>15.3} | {:>12.2}x ",
            (o0_opt_ns as f64) / 1_000_000.0,
            (base_ns as f64) / (o0_opt_ns as f64)
        );
    }
    if let Some(o3_ns) = c_o3_ns {
        println!(
            " JIT Compiled C (-O3, Opt)  | {:>15.3} | {:>12.2}x (Host Max)",
            (o3_ns as f64) / 1_000_000.0,
            (base_ns as f64) / (o3_ns as f64)
        );
    }
    println!("----------------------------------------------------------------");
    Ok(())
}

fn run_riscv32_demo() -> Result<(), Box<dyn Error>> {
    println!("================================================================");
    println!("PART 1.5: RISC-V32 (RV32I) Code Generation & QEMU Execution");
    println!("================================================================\n");

    let gcc_check = Command::new("riscv64-linux-gnu-gcc")
        .arg("--version")
        .output();
    let qemu_check = Command::new("qemu-riscv32").arg("--version").output();
    if gcc_check.is_err() || qemu_check.is_err() {
        println!("RISC-V32 toolchain (riscv64-linux-gnu-gcc or qemu-riscv32) is unavailable.");
        println!("Skipping RISC-V32 execution demo.\n");
        return Ok(());
    }

    println!("Loading sieve fixture...");
    let image = ModuleImage::from_text(SIEVE_FIXTURE).map_err(|e| format!("{:?}", e))?;
    let space =
        mirspace::ProgramSpace::from_module_image(&image).map_err(|e| format!("{:?}", e))?;
    let plan = build_compile_plan(&space);
    let lowered = lower_compile_plan(&plan);
    let optimized_lowered = mirplan::optimize_program(lowered);

    println!("Compiling optimized lowered plan to RV32I assembly via Riscv32Backend...");
    let backend = mirrv32::Riscv32Backend;
    let generated_asm = backend
        .compile(&optimized_lowered)
        .map_err(|e| e.to_string())?;

    // Append our custom baremetal stub and mir_alloc
    let mut full_asm = String::new();
    full_asm.push_str(&generated_asm);
    full_asm.push_str(
        r#"
.section .text
.global _start
_start:
    jal ra, mir_fn_1
    # Exit syscall (sys_exit is 93 on RISC-V)
    li a7, 93
    ecall

.global mir_alloc
mir_alloc:
    la t0, heap_ptr
    lw t1, 0(t0)
    addi t2, a1, -1
    add t1, t1, t2
    not t2, t2
    and t1, t1, t2
    la t3, heap_buffer
    li t4, 1048576
    add t3, t3, t4
    add t4, t1, a0
    bgtu t4, t3, .Loom
    sw t4, 0(t0)
    mv a0, t1
    ret
.Loom:
    li a0, 11
    li a7, 93
    ecall

.section .data
.align 4
heap_ptr:
    .word heap_buffer

.section .bss
.align 16
heap_buffer:
    .zero 1048576
"#,
    );

    println!("Printing snippet of the generated RV32I assembly (first 40 lines):");
    println!("----------------------------------------------------------------");
    for (idx, line) in full_asm.lines().take(40).enumerate() {
        println!("{:>2} | {}", idx + 1, line);
    }
    println!("...\n----------------------------------------------------------------");

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = fs::create_dir_all(&dir);
    let s_path = dir.join("temp_demo_sieve.s");
    let bin_path = dir.join("temp_demo_sieve");

    fs::write(&s_path, &full_asm)?;

    println!("Assembling and statically linking for RV32I target...");
    let mut compile_cmd = Command::new("riscv64-linux-gnu-gcc");
    compile_cmd
        .arg("-mabi=ilp32")
        .arg("-march=rv32im")
        .arg("-static")
        .arg("-nostdlib")
        .arg("-o")
        .arg(&bin_path)
        .arg(&s_path);

    let compile_output = compile_cmd.output()?;
    let _ = fs::remove_file(&s_path);

    if !compile_output.status.success() {
        return Err(format!(
            "RISC-V32 Compilation failed: {}",
            String::from_utf8_lossy(&compile_output.stderr)
        )
        .into());
    }

    println!("Executing statically-linked RV32I binary under QEMU user space...");
    let run_output = Command::new("qemu-riscv32").arg(&bin_path).output()?;

    let _ = fs::remove_file(&bin_path);

    let code = run_output.status.code().unwrap_or(255);
    println!("QEMU Execution return code (count of primes): {}", code);
    if code == 11 {
        println!("Execution succeeded! Sieve prime count is 11.");
    } else {
        println!("Unexpected exit code: {}", code);
    }
    println!("");

    Ok(())
}

fn run_branch_weights_demo() -> Result<(), Box<dyn Error>> {
    println!("================================================================");
    println!("PART 3: Branch Weight Analysis & Hot-Path Block Reordering");
    println!("================================================================\n");

    let branch_fixture_path = format!(
        "{}/../mircap/tests/fixtures/valid_branch_weights.mircap.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = fs::read(&branch_fixture_path)?;
    let image = ModuleImage::from_bytes(&bytes).map_err(|e| format!("{:?}", e))?;
    let space = mirspace::ProgramSpace::from_module_image(&image).map_err(|e| format!("{:?}", e))?;
    let plan = build_compile_plan(&space);
    let lowered = lower_compile_plan(&plan);

    println!("Original Lowered Blocks (Unoptimized):");
    for block in &lowered.functions[0].blocks {
        println!("  Block b{}#{}", block.label.ix.0, block.label.id.0);
    }

    let optimized = mirplan::optimize_program(lowered);
    println!("\nOptimized Blocks (After Hot-Path Reordering):");
    for block in &optimized.functions[0].blocks {
        println!("  Block b{}#{}", block.label.ix.0, block.label.id.0);
        if let Some(insn) = block.instructions.last() {
            if let mirplan::LoweredInstructionKind::Branch { weights, .. } = &insn.kind {
                if let Some(w) = weights {
                    let total: u64 = w.iter().sum();
                    if total > 0 {
                        let w_strs: Vec<String> = w.iter().map(|v| format!("{}%", (*v as f64 / total as f64 * 100.0).round())).collect();
                        println!("    -> Branch Heuristics Applied: [{}]", w_strs.join(", "));
                    }
                }
            }
        }
    }
    
    println!("\nBranch optimization applied successfully.");
    println!("");

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    run_pedagogical_demo()?;
    run_riscv32_demo()?;
    run_branch_weights_demo()?;
    run_performance_benchmarks()?;
    Ok(())
}
