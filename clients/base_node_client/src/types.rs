//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use taiji_common_types::types::{FixedHash, PublicKey};
use taiji_core::transactions::{taiji_amount::MicroMinotaiji, transaction_components::TransactionOutput};
use taiji_dan_common_types::{Epoch, ShardId};

#[derive(Debug, Clone)]
pub struct BaseLayerMetadata {
    pub height_of_longest_chain: u64,
    pub tip_hash: FixedHash,
}

#[derive(Debug, Clone)]
pub struct SideChainUtxos {
    pub block_info: BlockInfo,
    pub outputs: Vec<TransactionOutput>,
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub hash: FixedHash,
    pub height: u64,
    pub next_block_hash: Option<FixedHash>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidatorNode {
    pub public_key: PublicKey,
    pub shard_key: ShardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseLayerConsensusConstants {
    pub validator_node_registration_expiry: u64,
    pub epoch_length: u64,
    pub validator_node_registration_min_deposit_amount: MicroMinotaiji,
}

impl BaseLayerConsensusConstants {
    pub fn height_to_epoch(&self, height: u64) -> Epoch {
        Epoch(height / self.epoch_length)
    }

    pub fn epoch_to_height(&self, epoch: Epoch) -> u64 {
        epoch.0 * self.epoch_length
    }

    pub fn validator_node_registration_expiry(&self) -> Epoch {
        Epoch(self.validator_node_registration_expiry)
    }

    pub fn validator_node_registration_min_deposit_amount(&self) -> MicroMinotaiji {
        self.validator_node_registration_min_deposit_amount
    }

    pub fn epoch_length(&self) -> u64 {
        self.epoch_length
    }
}
