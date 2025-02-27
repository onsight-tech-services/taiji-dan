//   Copyright 2023 The Tari Project
//   SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use tari_common_types::types::FixedHash;
use tari_dan_common_types::{shard_bucket::ShardBucket, vn_node_hash, Epoch, NodeAddressable, ShardId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorNode<TAddr> {
    pub address: TAddr,
    pub shard_key: ShardId,
    pub epoch: Epoch,
    pub committee_bucket: Option<ShardBucket>,
    pub fee_claim_public_key: TAddr,
}

impl<TAddr: NodeAddressable> ValidatorNode<TAddr> {
    pub fn node_hash(&self) -> FixedHash {
        vn_node_hash(&self.address, &self.shard_key)
    }
}

impl<TAddr> PartialEq for ValidatorNode<TAddr> {
    fn eq(&self, other: &Self) -> bool {
        self.shard_key == other.shard_key
    }
}

impl<TAddr> Eq for ValidatorNode<TAddr> {}

impl<TAddr> std::hash::Hash for ValidatorNode<TAddr> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.shard_key.hash(state);
    }
}
