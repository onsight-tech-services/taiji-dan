//  Copyright 2022, The Tari Project
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

// use serde::Serialize;
// use tari_common_types::types::FixedHash;
// use tari_dan_common_types::{vn_node_hash, NodeAddressable, ShardId};
//
// #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
// pub struct ValidatorNode<TAddr> {
//     pub shard_key: ShardId,
//     pub public_key: TAddr,
// }
//
// impl<TAddr: NodeAddressable> ValidatorNode<TAddr> {
//     pub fn node_hash(&self) -> FixedHash {
//         vn_node_hash(&self.public_key, &self.shard_key)
//     }
// }
use tari_common_types::types::FixedHash;
use tari_dan_common_types::{vn_node_hash, Epoch, NodeAddressable, ShardId};

#[derive(Debug, Clone)]
pub struct ValidatorNode<TAddr> {
    pub address: TAddr,
    pub shard_key: ShardId,
    pub epoch: Epoch,
    pub committee_bucket: Option<u32>,
}

impl<TAddr: NodeAddressable> ValidatorNode<TAddr> {
    pub fn node_hash(&self) -> FixedHash {
        vn_node_hash(&self.address, &self.shard_key)
    }
}
