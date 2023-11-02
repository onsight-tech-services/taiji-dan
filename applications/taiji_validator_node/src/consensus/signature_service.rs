//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use std::sync::Arc;

use rand::rngs::OsRng;
use taiji_common_types::types::{FixedHash, PublicKey};
use taiji_comms::NodeIdentity;
use taiji_consensus::traits::{ValidatorSignatureService, VoteSignatureService};
use taiji_dan_storage::consensus_models::{BlockId, QuorumDecision, ValidatorSchnorrSignature, ValidatorSignature};

#[derive(Debug, Clone)]
pub struct TaijiSignatureService {
    node_identity: Arc<NodeIdentity>,
}

impl TaijiSignatureService {
    pub fn new(node_identity: Arc<NodeIdentity>) -> Self {
        Self { node_identity }
    }
}

impl ValidatorSignatureService<PublicKey> for TaijiSignatureService {
    fn sign<M: AsRef<[u8]>>(&self, message: M) -> ValidatorSchnorrSignature {
        ValidatorSchnorrSignature::sign_message(self.node_identity.secret_key(), message, &mut OsRng).unwrap()
    }

    fn public_key(&self) -> &PublicKey {
        self.node_identity.public_key()
    }
}

impl VoteSignatureService<PublicKey> for TaijiSignatureService {
    fn verify(
        &self,
        signature: &ValidatorSignature<PublicKey>,
        leaf_hash: &FixedHash,
        block_id: &BlockId,
        decision: &QuorumDecision,
    ) -> bool {
        let challenge = self.create_challenge(leaf_hash, block_id, decision);
        signature.verify(challenge)
    }
}
