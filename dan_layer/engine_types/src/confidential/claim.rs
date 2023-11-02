//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use taiji_common_types::types::PublicKey;
use tari_crypto::ristretto::RistrettoComSig;
use taiji_template_lib::models::{ConfidentialWithdrawProof, UnclaimedConfidentialOutputAddress};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct ConfidentialClaim {
    pub public_key: PublicKey,
    pub output_address: UnclaimedConfidentialOutputAddress,
    pub range_proof: Vec<u8>,
    pub proof_of_knowledge: RistrettoComSig,
    pub withdraw_proof: Option<ConfidentialWithdrawProof>,
}
