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

use serde::{Deserialize, Serialize};
use tari_bor::BorTag;
use tari_template_abi::{call_engine, rust::fmt, EngineOp};

use crate::{
    args::{BucketAction, BucketInvokeArg, BucketRef, InvokeResult},
    models::{Amount, BinaryTag, ConfidentialWithdrawProof, Proof, ResourceAddress},
    prelude::ResourceType,
};

const TAG: u64 = BinaryTag::BucketId.as_u64();

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct BucketId(BorTag<u32, TAG>);

impl From<u32> for BucketId {
    fn from(value: u32) -> Self {
        Self(BorTag::new(value))
    }
}

impl fmt::Display for BucketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BucketId({})", self.0.inner())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bucket {
    id: BucketId,
}

impl Bucket {
    pub const fn from_id(id: BucketId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> BucketId {
        self.id
    }

    pub fn resource_address(&self) -> ResourceAddress {
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::GetResourceAddress,
            args: invoke_args![],
        });

        resp.decode()
            .expect("Bucket GetResourceAddress returned invalid resource address")
    }

    pub fn resource_type(&self) -> ResourceType {
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::GetResourceType,
            args: invoke_args![],
        });

        resp.decode()
            .expect("Bucket GetResourceType returned invalid resource type")
    }

    pub fn take(&mut self, amount: Amount) -> Self {
        assert!(!amount.is_zero() && amount.is_positive());
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::Take,
            args: invoke_args![amount],
        });

        resp.decode().expect("Bucket Take returned invalid bucket")
    }

    pub fn take_confidential(&mut self, proof: ConfidentialWithdrawProof) -> Self {
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::TakeConfidential,
            args: invoke_args![proof],
        });

        resp.decode().expect("Bucket Take returned invalid bucket")
    }

    pub fn burn(&mut self) {
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::Burn,
            args: invoke_args![],
        });

        resp.decode().expect("Bucket Burn returned invalid result")
    }

    pub fn split(mut self, amount: Amount) -> (Self, Self) {
        let new_bucket = self.take(amount);
        (new_bucket, self)
    }

    pub fn split_confidential(mut self, proof: ConfidentialWithdrawProof) -> (Self, Self) {
        let new_bucket = self.take_confidential(proof);
        (new_bucket, self)
    }

    pub fn amount(&self) -> Amount {
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::GetAmount,
            args: invoke_args![],
        });

        resp.decode().expect("Bucket GetAmount returned invalid amount")
    }

    pub fn reveal_confidential(&mut self, proof: ConfidentialWithdrawProof) -> Bucket {
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::RevealConfidential,
            args: invoke_args![proof],
        });

        resp.decode()
            .expect("Bucket RevealConfidential returned invalid result")
    }

    pub fn create_proof(&self) -> Proof {
        let resp: InvokeResult = call_engine(EngineOp::BucketInvoke, &BucketInvokeArg {
            bucket_ref: BucketRef::Ref(self.id),
            action: BucketAction::CreateProof,
            args: invoke_args![],
        });

        resp.decode().expect("Bucket CreateProof returned invalid proof")
    }
}
