use mircap::{FunctionId, ModuleImage};
use std::error::Error;
use std::sync::{Arc, Mutex};

pub type CompilerHook = Arc<
    dyn Fn(&ModuleImage, FunctionId) -> Result<String, Box<dyn Error + Send + Sync>> + Send + Sync,
>;

#[derive(Clone)]
pub enum ThunkTarget {
    Interpreter,
    LazyCompile { hook: CompilerHook },
    Compiled { binary_path: String },
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
