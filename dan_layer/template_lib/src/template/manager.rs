//   Copyright 2023. OnSight Tech Services LLC
//
//   Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//   following conditions are met:
//
//   1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//   disclaimer.
//
//   2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//   following disclaimer in the documentation and/or other materials provided with the distribution.
//
//   3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//   products derived from this software without specific prior written permission.
//
//   THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//   INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//   DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//   SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//   SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//   WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//   USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use serde::de::DeserializeOwned;
use taiji_template_abi::{call_engine, EngineOp};

use crate::{
    args::{CallAction, CallFunctionArg, CallInvokeArg, InvokeResult},
    prelude::TemplateAddress,
};

#[derive(Debug)]
pub struct TemplateManager {
    template_address: TemplateAddress,
}

impl TemplateManager {
    pub fn get(template_address: TemplateAddress) -> Self {
        Self { template_address }
    }

    pub fn call<T: DeserializeOwned>(&self, function: String, args: Vec<Vec<u8>>) -> T {
        self.call_internal(CallFunctionArg {
            template_address: self.template_address,
            function,
            args,
        })
    }

    fn call_internal<T: DeserializeOwned>(&self, arg: CallFunctionArg) -> T {
        let result = call_engine::<_, InvokeResult>(EngineOp::CallInvoke, &CallInvokeArg {
            action: CallAction::CallFunction,
            args: invoke_args![arg],
        });

        result
            .decode()
            .expect("failed to decode template function call result from engine")
    }
}
