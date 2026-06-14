use mircap::{FunctionId, ModuleImage};
use std::error::Error;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Arc, Mutex};

extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *mut c_char;
}

pub struct DynamicLibrary {
    handle: *mut c_void,
}

impl DynamicLibrary {
    pub fn new(path: &str) -> Result<Self, String> {
        let path_c = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
        unsafe {
            dlerror();
            let handle = dlopen(path_c.as_ptr(), 2); // RTLD_NOW = 2
            if handle.is_null() {
                let err = dlerror();
                let err_str = if err.is_null() {
                    "Unknown dlopen error".to_string()
                } else {
                    std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned()
                };
                return Err(format!("dlopen failed: {}", err_str));
            }
            Ok(Self { handle })
        }
    }

    pub fn get_symbol(&self, name: &str) -> Result<*mut c_void, String> {
        let name_c = std::ffi::CString::new(name).map_err(|e| e.to_string())?;
        unsafe {
            dlerror();
            let sym = dlsym(self.handle, name_c.as_ptr());
            if sym.is_null() {
                let err = dlerror();
                if !err.is_null() {
                    let err_str = std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned();
                    return Err(format!("dlsym failed for {}: {}", name, err_str));
                }
            }
            Ok(sym)
        }
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        unsafe {
            dlclose(self.handle);
        }
    }
}

unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

pub type CompilerHook = Arc<
    dyn Fn(&ModuleImage, FunctionId) -> Result<String, Box<dyn Error + Send + Sync>> + Send + Sync,
>;

#[derive(Clone)]
pub enum ThunkTarget {
    Interpreter,
    LazyCompile {
        hook: CompilerHook,
    },
    Compiled {
        binary_path: String,
    },
    InProcess {
        binary_path: String,
        handle: Arc<DynamicLibrary>,
    },
}

#[derive(Clone)]
pub struct Thunk {
    pub function_id: FunctionId,
    pub name: String,
    pub target: Arc<Mutex<ThunkTarget>>,
}

impl Thunk {
    pub fn new(function_id: FunctionId, name: String, target: ThunkTarget) -> Self {
        Self {
            function_id,
            name,
            target: Arc::new(Mutex::new(target)),
        }
    }

    pub fn target(&self) -> ThunkTarget {
        self.target.lock().unwrap().clone()
    }

    pub fn set_target(&self, new_target: ThunkTarget) {
        *self.target.lock().unwrap() = new_target;
    }
}
