//  Copyright 2022. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::sync::Arc;

use tari_template_abi::{FunctionDef, TemplateDef, ABI_TEMPLATE_DEF_GLOBAL_NAME};
use wasmer::{
    BaseTunables,
    CompilerConfig,
    Cranelift,
    CraneliftOptLevel,
    Engine,
    ExportError,
    Function,
    Instance,
    Store,
    Universal,
    WasmerEnv,
};

use crate::{
    packager::{LoadedTemplate, PackageError, TemplateModuleLoader},
    wasm::{environment::WasmEnv, metering, WasmExecutionError},
};

#[derive(Debug, Clone)]
pub struct WasmModule {
    code: Vec<u8>,
}

impl WasmModule {
    pub fn from_code(code: Vec<u8>) -> Self {
        Self { code }
    }

    pub fn load_template_from_code(code: &[u8]) -> Result<LoadedTemplate, PackageError> {
        let store = Self::create_store();
        let module = wasmer::Module::new(&store, code)?;
        let mut env = WasmEnv::new(());
        fn stub(_env: &WasmEnv<()>, _op: i32, _arg_ptr: i32, _arg_len: i32) -> i32 {
            0
        }

        let stub = Function::new_native_with_env(&store, env.clone(), stub);
        let imports = env.create_resolver(&store, stub);
        let instance = Instance::new(&module, &imports)?;
        env.init_with_instance(&instance)?;
        validate_instance(&instance)?;
        validate_environment(&env)?;

        let template = initialize_and_load_template_abi(&instance, &env)?;
        Ok(LoadedWasmTemplate::new(template, module, code.len()).into())
    }

    pub fn code(&self) -> &[u8] {
        &self.code
    }

    fn create_store() -> Store {
        let mut cranelift = Cranelift::new();
        cranelift.opt_level(CraneliftOptLevel::Speed).canonicalize_nans(true);
        // TODO: Configure metering limit
        cranelift.push_middleware(Arc::new(metering::middleware(100_000_000)));
        let engine = Universal::new(cranelift).engine();
        let tunables = BaseTunables::for_target(engine.target());
        Store::new_with_tunables(&engine, tunables)
    }
}

impl TemplateModuleLoader for WasmModule {
    fn load_template(&self) -> Result<LoadedTemplate, PackageError> {
        Self::load_template_from_code(&self.code)
    }
}

fn initialize_and_load_template_abi<T: Clone + Send + Sync + 'static>(
    instance: &Instance,
    env: &WasmEnv<T>,
) -> Result<TemplateDef, WasmExecutionError> {
    let ptr = instance
        .exports
        .get_global(ABI_TEMPLATE_DEF_GLOBAL_NAME)?
        .get()
        .i32()
        .ok_or(WasmExecutionError::ExportError(ExportError::IncompatibleType))?;
    let ptr = u32::try_from(ptr).map_err(|_| WasmExecutionError::ExportError(ExportError::IncompatibleType))?;

    // Load ABI from memory
    let data = env.read_memory_with_embedded_len(ptr)?;
    let decoded = tari_bor::decode(&data).map_err(WasmExecutionError::AbiDecodeError)?;
    Ok(decoded)
}

#[derive(Debug, Clone)]
pub struct LoadedWasmTemplate {
    template_def: Arc<TemplateDef>,
    module: wasmer::Module,
    code_size: usize,
}

impl LoadedWasmTemplate {
    pub fn new(template_def: TemplateDef, module: wasmer::Module, code_size: usize) -> Self {
        Self {
            template_def: Arc::new(template_def),
            module,
            code_size,
        }
    }

    pub fn wasm_module(&self) -> &wasmer::Module {
        &self.module
    }

    pub fn template_name(&self) -> &str {
        self.template_def.template_name()
    }

    pub fn template_def(&self) -> &TemplateDef {
        &self.template_def
    }

    pub fn find_func_by_name(&self, function_name: &str) -> Option<&FunctionDef> {
        self.template_def.functions().iter().find(|f| f.name == *function_name)
    }

    pub fn code_size(&self) -> usize {
        self.code_size
    }
}

fn validate_environment<T: Clone + Send + Sync + 'static>(env: &WasmEnv<T>) -> Result<(), WasmExecutionError> {
    const MAX_MEM_SIZE: usize = 2 * 1024 * 1024;
    let mem_size = env.mem_size();
    if mem_size.bytes().0 > MAX_MEM_SIZE {
        return Err(WasmExecutionError::MaxMemorySizeExceeded);
    }

    Ok(())
}

fn validate_instance(instance: &Instance) -> Result<(), WasmExecutionError> {
    // Enforce that only permitted functions are allowed
    let unexpected_abi_func = instance
        .exports
        .iter()
        .functions()
        .find(|(name, _)| !is_func_permitted(name));
    if let Some((name, _)) = unexpected_abi_func {
        return Err(WasmExecutionError::UnexpectedAbiFunction { name: name.to_string() });
    }

    instance
        .exports
        .get_global(ABI_TEMPLATE_DEF_GLOBAL_NAME)?
        .get()
        .i32()
        .ok_or(WasmExecutionError::ExportError(ExportError::IncompatibleType))?;

    Ok(())
}

fn is_func_permitted(name: &str) -> bool {
    name.ends_with("_main") || name == "tari_alloc" || name == "tari_free"
}
