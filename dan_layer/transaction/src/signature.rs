//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use taiji_common_types::types::{PublicKey, Signature};
use tari_crypto::{
    keys::PublicKey as PublicKeyT,
    ristretto::{RistrettoPublicKey, RistrettoSecretKey},
};
use taiji_engine_types::{
    hashing::{hasher, EngineHashDomainLabel},
    instruction::Instruction,
};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionSignature {
    public_key: PublicKey,
    signature: Signature,
}

impl TransactionSignature {
    pub fn new(public_key: PublicKey, signature: Signature) -> Self {
        Self { public_key, signature }
    }

    pub fn sign(secret_key: &RistrettoSecretKey, instructions: &[Instruction]) -> Self {
        let public_key = RistrettoPublicKey::from_secret_key(secret_key);
        let challenge = hasher(EngineHashDomainLabel::InstructionSignature)
            .chain(instructions)
            .result();

        Self {
            signature: Signature::sign_message(secret_key, challenge, &mut OsRng).unwrap(),
            public_key,
        }
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn public_key(&self) -> &RistrettoPublicKey {
        &self.public_key
    }
}
