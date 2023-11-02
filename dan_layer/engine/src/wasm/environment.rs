//  Copyright 2022. OnSight Tech Services LLC
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

use std::{
    cell::Cell,
    fmt::{Debug, Formatter},
    sync::{Arc, Mutex},
};

use wasmer::{
    imports,
    Function,
    HostEnvInitError,
    Instance,
    LazyInit,
    Memory,
    NativeFunc,
    Pages,
    Resolver,
    Store,
    WasmerEnv,
};

use crate::{runtime::RuntimeError, wasm::WasmExecutionError};

#[derive(Clone)]
pub struct WasmEnv<T> {
    memory: LazyInit<Memory>,
    mem_alloc: LazyInit<NativeFunc<i32, i32>>,
    mem_free: LazyInit<NativeFunc<i32>>,
    state: T,
    last_panic: Arc<Mutex<Option<String>>>,
    last_engine_error: Arc<Mutex<Option<RuntimeError>>>,
}

impl<T: Clone + Sync + Send + 'static> WasmEnv<T> {
    pub fn new(state: T) -> Self {
        Self {
            state,
            memory: LazyInit::new(),
            mem_alloc: LazyInit::new(),
            mem_free: LazyInit::new(),
            last_panic: Arc::new(Mutex::new(None)),
            last_engine_error: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn alloc(&self, len: u32) -> Result<AllocPtr, WasmExecutionError> {
        let ptr = self.get_mem_alloc_func()?.call(len as i32)?;
        if ptr == 0 {
            return Err(WasmExecutionError::MemoryAllocationFailed);
        }

        Ok(AllocPtr(ptr as u32, len))
    }

    pub(super) fn free(&self, ptr: AllocPtr) -> Result<(), WasmExecutionError> {
        self.get_mem_free_func()?.call(ptr.as_i32())?;
        Ok(())
    }

    pub(super) fn take_last_panic_message(&self) -> Option<String> {
        self.last_panic.lock().unwrap().take()
    }

    pub(super) fn set_last_engine_error(&self, error: RuntimeError) {
        *self.last_engine_error.lock().unwrap() = Some(error);
    }

    pub(super) fn take_last_engine_error(&self) -> Option<RuntimeError> {
        self.last_engine_error.lock().unwrap().take()
    }

    pub(super) fn write_to_memory(&self, ptr: &AllocPtr, data: &[u8]) -> Result<(), WasmExecutionError> {
        if data.len() != ptr.len() as usize {
            return Err(WasmExecutionError::InvalidWriteLength {
                allocated: ptr.len(),
                requested: data.len() as u32,
            });
        }
        // SAFETY: The pointer has been allocated by alloc above and the runtime is single-threaded so data
        // races are not possible.
        unsafe {
            self.get_memory()?
                .uint8view()
                .subarray(ptr.get(), ptr.end())
                .copy_from(data);
        }
        Ok(())
    }

    pub(super) fn read_memory_with_embedded_len(&self, ptr: u32) -> Result<Vec<u8>, WasmExecutionError> {
        let memory = self.get_memory()?;
        let view = memory.uint8view().subarray(ptr, memory.data_size() as u32 - 1);
        let view_bytes = &*view;
        if view_bytes.len() < 4 {
            return Err(WasmExecutionError::MemoryUnderflow {
                required: 4,
                remaining: view_bytes.len(),
            });
        }

        let mut buf = [0u8; 4];
        copy_from_cell_slice(view_bytes, &mut buf);
        let len = u32::from_le_bytes(buf);
        let data = self.read_from_memory(ptr + 4, len)?;

        Ok(data)
    }

    pub(super) fn read_from_memory(&self, ptr: u32, len: u32) -> Result<Vec<u8>, WasmExecutionError> {
        let memory = self.get_memory()?;
        let size = memory.data_size();
        if u64::from(ptr) >= size || u64::from(ptr + len) >= memory.data_size() {
            return Err(WasmExecutionError::MemoryPointerOutOfRange {
                size: memory.data_size(),
                pointer: u64::from(ptr),
                len: u64::from(len),
            });
        }
        let view = memory.uint8view().subarray(ptr, ptr + len);
        let mut data = vec![0u8; len as usize];
        copy_from_cell_slice(&view, &mut data);
        Ok(data)
    }

    pub fn state(&self) -> &T {
        &self.state
    }

    fn get_mem_alloc_func(&self) -> Result<&NativeFunc<i32, i32>, WasmExecutionError> {
        self.mem_alloc
            .get_ref()
            .ok_or_else(|| WasmExecutionError::MissingAbiFunction {
                function: "taiji_alloc".into(),
            })
    }

    fn get_mem_free_func(&self) -> Result<&NativeFunc<i32>, WasmExecutionError> {
        self.mem_free
            .get_ref()
            .ok_or_else(|| WasmExecutionError::MissingAbiFunction {
                function: "taiji_free".into(),
            })
    }

    fn get_memory(&self) -> Result<&Memory, WasmExecutionError> {
        self.memory.get_ref().ok_or(WasmExecutionError::MemoryNotInitialized)
    }

    pub fn mem_size(&self) -> Pages {
        self.memory.get_ref().map(|mem| mem.size()).unwrap_or(Pages(0))
    }

    pub fn create_resolver(&self, store: &Store, taiji_engine: Function) -> impl Resolver {
        imports! {
            "env" => {
                "taiji_engine" => taiji_engine,
                "debug" => Function::new_native_with_env(store, self.clone(), Self::debug_handler),
                "on_panic" => Function::new_native_with_env(store, self.clone(), Self::on_panic_handler),
            }
        }
    }

    fn debug_handler(env: &Self, arg_ptr: i32, arg_len: i32) {
        const WASM_DEBUG_LOG_TARGET: &str = "taiji::dan::wasm";
        match env.read_from_memory(arg_ptr as u32, arg_len as u32) {
            Ok(arg) => {
                eprintln!("DEBUG: {}", String::from_utf8_lossy(&arg));
            },
            Err(err) => {
                log::error!(target: WASM_DEBUG_LOG_TARGET, "Failed to read from memory: {}", err);
            },
        }
    }

    fn on_panic_handler(env: &Self, msg_ptr: i32, msg_len: i32, line: i32, col: i32) {
        const WASM_DEBUG_LOG_TARGET: &str = "taiji::dan::wasm";
        match env.read_from_memory(msg_ptr as u32, msg_len as u32) {
            Ok(msg) => {
                let msg = String::from_utf8_lossy(&msg);
                eprintln!("📣 PANIC: ({}:{}) {}", line, col, msg);
                log::error!(target: WASM_DEBUG_LOG_TARGET, "📣 PANIC: ({}:{}) {}", line, col, msg);
                *env.last_panic.lock().unwrap() = Some(msg.to_string());
            },
            Err(err) => {
                log::error!(
                    target: WASM_DEBUG_LOG_TARGET,
                    "📣 PANIC: WASM template panicked but did not provide a valid memory pointer to on_panic \
                     callback: {}",
                    err
                );
                *env.last_panic.lock().unwrap() = Some(format!(
                    "WASM panicked but did not provide a valid message pointer to on_panic callback: {}",
                    err
                ));
            },
        }
    }
}

impl<T: Clone + Sync + Send> WasmerEnv for WasmEnv<T> {
    fn init_with_instance(&mut self, instance: &Instance) -> Result<(), HostEnvInitError> {
        self.memory
            .initialize(instance.exports.get_with_generics_weak("memory")?);
        self.mem_alloc
            .initialize(instance.exports.get_with_generics_weak("taiji_alloc")?);
        self.mem_free
            .initialize(instance.exports.get_with_generics_weak("taiji_free")?);
        Ok(())
    }
}

impl<T: Debug> Debug for WasmEnv<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmEnv")
            .field("memory", &"LazyInit<Memory>")
            .field("taiji_alloc", &" LazyInit<NativeFunc<(i32), (i32)>")
            .field("taiji_free", &"LazyInit<NativeFunc<(i32, i32), ()>>")
            .field("State", &self.state)
            .finish()
    }
}

/// Efficiently copy read-only memory into a mutable buffer.
/// Panics if the length of `dest` is more than the length of `src`.
fn copy_from_cell_slice(src: &[Cell<u8>], dest: &mut [u8]) {
    assert!(dest.len() <= src.len());
    let len = dest.len();
    // SAFETY: size_of::<Cell<u8>() is equal to size_of::<u8>(), we assert this below just in case.
    let (head, body, tail) = unsafe { src[..len].align_to() };
    assert_eq!(head.len(), 0);
    assert_eq!(tail.len(), 0);
    dest.copy_from_slice(body);
}

#[derive(Debug)]
pub struct AllocPtr(u32, u32);

impl AllocPtr {
    pub fn get(&self) -> u32 {
        self.0
    }

    pub fn len(&self) -> u32 {
        self.1
    }

    pub fn end(&self) -> u32 {
        self.get() + self.len()
    }

    pub fn as_i32(&self) -> i32 {
        // We want the 'u32 as i32' conversion to wrap
        self.get() as i32
    }
}
