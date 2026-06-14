use crate::thunk::{CompilerHook, Thunk, ThunkTarget};
use mircap::{FunctionId, ModuleImage};
use mirsem::error::RunError;
use mirsem::profile::ExecutionProfile;
use mirsem::runner::{ExecutionResult, Runner};
use mirsem::trap::ExecutionTrap;
use mirsem::value::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::process::Command;

#[derive(Debug)]
pub enum JitError {
    Interpreter(mirsem::error::ExecutionError),
    InterpreterRun(RunError),
    Compile(String),
    Io(std::io::Error),
    ProcessFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    Trap(ExecutionTrap),
    SymbolNotFound(String),
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitError::Interpreter(e) => write!(f, "Interpreter error: {:?}", e),
            JitError::InterpreterRun(e) => write!(f, "Interpreter run error: {:?}", e),
            JitError::Compile(e) => write!(f, "Compile error: {}", e),
            JitError::Io(e) => write!(f, "IO error: {}", e),
            JitError::ProcessFailed { exit_code, stderr } => {
                write!(f, "Process failed with code {:?}: {}", exit_code, stderr)
            }
            JitError::Trap(t) => write!(f, "Execution trap: {:?}", t),
            JitError::SymbolNotFound(name) => write!(f, "Symbol not found: {}", name),
        }
    }
}

impl std::error::Error for JitError {}

pub struct JitContext {
    pub image: ModuleImage,
    pub profile: ExecutionProfile,
    pub thunks: HashMap<FunctionId, Thunk>,
}

impl JitContext {
    pub fn new(image: ModuleImage, profile: ExecutionProfile) -> Self {
        let mut thunks = HashMap::new();
        for func in &image.functions {
            if let Some(sym) = image.symbol(func.symbol) {
                let thunk = Thunk::new(func.id, sym.name.clone(), ThunkTarget::Interpreter);
                thunks.insert(func.id, thunk);
            }
        }
        Self {
            image,
            profile,
            thunks,
        }
    }

    pub fn set_lazy_compile(&mut self, hook: CompilerHook) {
        for thunk in self.thunks.values_mut() {
            thunk.set_target(ThunkTarget::LazyCompile { hook: hook.clone() });
        }
    }

    pub fn set_eager_compile<F>(
        &mut self,
        compile_fn: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        F: Fn(&ModuleImage, FunctionId) -> Result<String, Box<dyn Error + Send + Sync>>,
    {
        for thunk in self.thunks.values() {
            let binary_path = compile_fn(&self.image, thunk.function_id)?;
            if binary_path.ends_with(".so") {
                let dl = crate::thunk::DynamicLibrary::new(&binary_path).map_err(|e| {
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                        as Box<dyn Error + Send + Sync>
                })?;
                thunk.set_target(ThunkTarget::InProcess {
                    binary_path,
                    handle: std::sync::Arc::new(dl),
                });
            } else {
                thunk.set_target(ThunkTarget::Compiled { binary_path });
            }
        }
        Ok(())
    }

    pub fn call_by_name(&self, name: &str, args: &[Value]) -> Result<ExecutionResult, JitError> {
        let thunk = self
            .thunks
            .values()
            .find(|t| t.name == name)
            .ok_or_else(|| JitError::SymbolNotFound(name.to_string()))?;
        self.call_thunk(thunk, args)
    }

    pub fn call_thunk(&self, thunk: &Thunk, args: &[Value]) -> Result<ExecutionResult, JitError> {
        let target = thunk.target();
        match target {
            ThunkTarget::Interpreter => {
                let mut runner = Runner::new(self.image.clone(), self.profile.clone())
                    .map_err(JitError::Interpreter)?;
                runner
                    .run_entry(thunk.function_id, args)
                    .map_err(JitError::InterpreterRun)
            }
            ThunkTarget::LazyCompile { hook } => {
                let path = hook(&self.image, thunk.function_id)
                    .map_err(|e| JitError::Compile(e.to_string()))?;
                if path.ends_with(".so") {
                    let dl = crate::thunk::DynamicLibrary::new(&path)
                        .map_err(|e| JitError::Compile(e))?;
                    let target = ThunkTarget::InProcess {
                        binary_path: path.clone(),
                        handle: std::sync::Arc::new(dl),
                    };
                    thunk.set_target(target.clone());
                    match target {
                        ThunkTarget::InProcess { handle, .. } => unsafe {
                            self.run_in_process(&handle, thunk.function_id, args)
                        },
                        _ => unreachable!(),
                    }
                } else {
                    thunk.set_target(ThunkTarget::Compiled {
                        binary_path: path.clone(),
                    });
                    self.run_compiled_binary(&path)
                }
            }
            ThunkTarget::Compiled { binary_path } => self.run_compiled_binary(&binary_path),
            ThunkTarget::InProcess { handle, .. } => unsafe {
                self.run_in_process(&handle, thunk.function_id, args)
            },
        }
    }

    fn run_compiled_binary(&self, binary_path: &str) -> Result<ExecutionResult, JitError> {
        let output = Command::new(binary_path).output().map_err(JitError::Io)?;

        if output.status.success() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let result_line = stdout_str.lines().find(|l| l.starts_with("Result: "));

            let val = match result_line {
                Some("Result: void") => Value::Void,
                Some(line) if line.starts_with("Result: i32 ") => {
                    let val_str = &line["Result: i32 ".len()..];
                    let val = val_str
                        .parse::<i32>()
                        .map_err(|_| JitError::Compile("Failed to parse i32".to_string()))?;
                    Value::I32(val)
                }
                Some(line) if line.starts_with("Result: u32 ") => {
                    let val_str = &line["Result: u32 ".len()..];
                    let val = val_str
                        .parse::<u32>()
                        .map_err(|_| JitError::Compile("Failed to parse u32".to_string()))?;
                    Value::U32(val)
                }
                Some(line) if line.starts_with("Result: addr32 ") => {
                    let val_str = &line["Result: addr32 ".len()..];
                    let val = val_str
                        .parse::<u32>()
                        .map_err(|_| JitError::Compile("Failed to parse addr32".to_string()))?;
                    Value::Addr32(val)
                }
                _ => Value::Void,
            };

            Ok(ExecutionResult {
                values: vec![val],
                executed_instruction_count: 0,
            })
        } else {
            let code = output.status.code().unwrap_or(99);
            let stderr_str = String::from_utf8_lossy(&output.stderr);

            let trap = match code {
                1 => ExecutionTrap::StackOverflow {
                    max_depth: self.profile.max_call_depth,
                },
                2 => ExecutionTrap::FuelExhausted {
                    max_instructions: self.profile.max_instructions,
                },
                3 => ExecutionTrap::ExplicitTrap {
                    instruction: mircap::InstructionId(0),
                },
                11 => ExecutionTrap::OutOfMemory {
                    requested: 0,
                    align: 0,
                },
                12 => ExecutionTrap::HeapStackCollision {
                    requested: 0,
                    align: 0,
                },
                13 => ExecutionTrap::OutOfBoundsLoad { addr: 0, size: 0 },
                14 => ExecutionTrap::OutOfBoundsStore { addr: 0, size: 0 },
                15 => ExecutionTrap::MisalignedLoad { addr: 0, align: 0 },
                16 => ExecutionTrap::MisalignedStore { addr: 0, align: 0 },
                17 => ExecutionTrap::AddressOverflow { base: 0, offset: 0 },
                _ => {
                    return Err(JitError::ProcessFailed {
                        exit_code: Some(code),
                        stderr: stderr_str.to_string(),
                    });
                }
            };

            Err(JitError::Trap(trap))
        }
    }

    unsafe fn run_in_process(
        &self,
        handle: &std::sync::Arc<crate::thunk::DynamicLibrary>,
        function_id: mircap::FunctionId,
        args: &[Value],
    ) -> Result<ExecutionResult, JitError> {
        // 1. Reset heap pointer
        let heap_ptr_sym = handle
            .get_symbol("g_heap_ptr")
            .map_err(|e| JitError::Compile(e))?;
        let g_heap_ptr_ref = heap_ptr_sym as *mut u32;
        *g_heap_ptr_ref = 0;

        // 2. Initialize data segments
        let init_data_sym = handle
            .get_symbol("init_data_segments")
            .map_err(|e| JitError::Compile(e))?;
        let init_data_segments: extern "C" fn() = std::mem::transmute(init_data_sym);
        init_data_segments();

        // 3. Resolve target function
        let fn_name = format!("mir_fn_{}", function_id.0);
        let fn_sym = handle
            .get_symbol(&fn_name)
            .map_err(|e| JitError::Compile(e))?;

        // 4. Resolve function signature and call it
        let func = self.image.function(function_id).ok_or_else(|| {
            JitError::Compile(format!("Function {} not found in image", function_id.0))
        })?;

        let param_kinds: Vec<mircap::TypeKind> = func
            .params
            .iter()
            .map(|&tid| self.image.type_kind(tid).unwrap_or(mircap::TypeKind::Void))
            .collect();
        let result_kinds: Vec<mircap::TypeKind> = func
            .results
            .iter()
            .map(|&tid| self.image.type_kind(tid).unwrap_or(mircap::TypeKind::Void))
            .collect();

        self.call_ffi(fn_sym, &param_kinds, &result_kinds, args)
    }

    unsafe fn call_ffi(
        &self,
        func_ptr: *mut std::os::raw::c_void,
        _param_kinds: &[mircap::TypeKind],
        result_kinds: &[mircap::TypeKind],
        args: &[Value],
    ) -> Result<ExecutionResult, JitError> {
        let mut arg_vals = Vec::new();
        for arg in args {
            match arg {
                Value::Void => arg_vals.push(0),
                Value::I32(v) => arg_vals.push(*v as u32),
                Value::U32(v) => arg_vals.push(*v),
                Value::Addr32(v) => arg_vals.push(*v),
                Value::I64(v) => {
                    arg_vals.push((*v & 0xFFFFFFFF) as u32);
                    arg_vals.push(((*v >> 32) & 0xFFFFFFFF) as u32);
                }
            }
        }

        let ret_val = match (arg_vals.len(), result_kinds.first()) {
            (0, None) => {
                let f: extern "C" fn() = std::mem::transmute(func_ptr);
                f();
                Value::Void
            }
            (0, Some(mircap::TypeKind::I32)) => {
                let f: extern "C" fn() -> i32 = std::mem::transmute(func_ptr);
                Value::I32(f())
            }
            (0, Some(mircap::TypeKind::U32)) => {
                let f: extern "C" fn() -> u32 = std::mem::transmute(func_ptr);
                Value::U32(f())
            }
            (0, Some(mircap::TypeKind::Addr32)) => {
                let f: extern "C" fn() -> u32 = std::mem::transmute(func_ptr);
                Value::Addr32(f())
            }
            (0, Some(mircap::TypeKind::I64)) => {
                let f: extern "C" fn() -> i64 = std::mem::transmute(func_ptr);
                Value::I64(f())
            }
            (1, None) => {
                let f: extern "C" fn(u32) = std::mem::transmute(func_ptr);
                f(arg_vals[0]);
                Value::Void
            }
            (1, Some(mircap::TypeKind::I32)) => {
                let f: extern "C" fn(u32) -> i32 = std::mem::transmute(func_ptr);
                Value::I32(f(arg_vals[0]))
            }
            (1, Some(mircap::TypeKind::U32)) => {
                let f: extern "C" fn(u32) -> u32 = std::mem::transmute(func_ptr);
                Value::U32(f(arg_vals[0]))
            }
            (1, Some(mircap::TypeKind::Addr32)) => {
                let f: extern "C" fn(u32) -> u32 = std::mem::transmute(func_ptr);
                Value::Addr32(f(arg_vals[0]))
            }
            (1, Some(mircap::TypeKind::I64)) => {
                let f: extern "C" fn(u32) -> i64 = std::mem::transmute(func_ptr);
                Value::I64(f(arg_vals[0]))
            }
            (2, None) => {
                let f: extern "C" fn(u32, u32) = std::mem::transmute(func_ptr);
                f(arg_vals[0], arg_vals[1]);
                Value::Void
            }
            (2, Some(mircap::TypeKind::I32)) => {
                let f: extern "C" fn(u32, u32) -> i32 = std::mem::transmute(func_ptr);
                Value::I32(f(arg_vals[0], arg_vals[1]))
            }
            (2, Some(mircap::TypeKind::U32)) => {
                let f: extern "C" fn(u32, u32) -> u32 = std::mem::transmute(func_ptr);
                Value::U32(f(arg_vals[0], arg_vals[1]))
            }
            (2, Some(mircap::TypeKind::Addr32)) => {
                let f: extern "C" fn(u32, u32) -> u32 = std::mem::transmute(func_ptr);
                Value::Addr32(f(arg_vals[0], arg_vals[1]))
            }
            (2, Some(mircap::TypeKind::I64)) => {
                let f: extern "C" fn(u32, u32) -> i64 = std::mem::transmute(func_ptr);
                Value::I64(f(arg_vals[0], arg_vals[1]))
            }
            _ => return Err(JitError::Compile("Unsupported FFI signature".to_string())),
        };

        Ok(ExecutionResult {
            values: vec![ret_val],
            executed_instruction_count: 0,
        })
    }
}
