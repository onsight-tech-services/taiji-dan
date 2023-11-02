//  Copyright 2022 OnSight Tech Services LLC
//  SPDX-License-Identifier: BSD-3-Clause
use taiji_template_abi::{call_engine, EngineOp};

use crate::{
    args::{CallerContextAction, CallerContextInvokeArg, InvokeResult},
    crypto::RistrettoPublicKeyBytes,
};

pub struct CallerContext {}

impl CallerContext {
    pub fn caller() -> RistrettoPublicKeyBytes {
        let resp: InvokeResult = call_engine(EngineOp::CallerContextInvoke, &CallerContextInvokeArg {
            action: CallerContextAction::GetCallerPublicKey,
        });

        resp.decode().expect("Failed to decode PublicKey")
    }
}
