//   Copyright 2023 The Tari Project
//   SPDX-License-Identifier: BSD-3-Clause

use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
};

use async_trait::async_trait;
use tari_base_node_client::types::BaseLayerConsensusConstants;
use tari_common_types::types::{FixedHash, PublicKey};
use tari_comms::types::CommsPublicKey;
use tari_core::transactions::transaction_components::ValidatorNodeRegistration;
use tari_dan_common_types::{
    committee::{Committee, CommitteeShard},
    hashing::MergedValidatorNodeMerkleProof,
    shard_bucket::ShardBucket,
    Epoch,
    ShardId,
};
use tari_dan_storage::global::models::ValidatorNode;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::{
    base_layer::types::EpochManagerRequest,
    error::EpochManagerError,
    traits::EpochManagerReader,
    EpochManagerEvent,
};

#[derive(Clone, Debug)]
pub struct EpochManagerHandle {
    tx_request: mpsc::Sender<EpochManagerRequest>,
}

impl EpochManagerHandle {
    pub fn new(tx_request: mpsc::Sender<EpochManagerRequest>) -> Self {
        Self { tx_request }
    }

    pub async fn update_epoch(&self, block_height: u64, block_hash: FixedHash) -> Result<(), EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::UpdateEpoch {
                block_height,
                block_hash,
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;
        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn get_base_layer_consensus_constants(&self) -> Result<BaseLayerConsensusConstants, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetBaseLayerConsensusConstants { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;
        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn last_registration_epoch(&self) -> Result<Option<Epoch>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::LastRegistrationEpoch { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;
        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn update_last_registration_epoch(&self, epoch: Epoch) -> Result<(), EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::UpdateLastRegistrationEpoch { epoch, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    /// Returns the number of epochs remaining for the current registration if registered, otherwise None
    pub async fn remaining_registration_epochs(&self) -> Result<Option<Epoch>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::RemainingRegistrationEpochs { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;
        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn add_validator_node_registration(
        &self,
        block_height: u64,
        registration: ValidatorNodeRegistration,
    ) -> Result<(), EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::AddValidatorNodeRegistration {
                block_height,
                registration,
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;
        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn current_block_height(&self) -> Result<u64, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::CurrentBlockHeight { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn notify_scanning_complete(&self) -> Result<(), EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::NotifyScanningComplete { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn get_fee_claim_public_key(&self) -> Result<Option<PublicKey>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetFeeClaimPublicKey { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn set_fee_claim_public_key(&self, public_key: PublicKey) -> Result<(), EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::SetFeeClaimPublicKey { public_key, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn get_all_validator_nodes(
        &self,
        epoch: Epoch,
    ) -> Result<Vec<ValidatorNode<CommsPublicKey>>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetValidatorNodesPerEpoch { epoch, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    pub async fn get_committees_by_shards(
        &self,
        epoch: Epoch,
        shards: HashSet<ShardId>,
    ) -> Result<HashMap<ShardBucket, Committee<CommsPublicKey>>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetCommittees {
                epoch,
                shards,
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }
}

#[async_trait]
impl EpochManagerReader for EpochManagerHandle {
    type Addr = CommsPublicKey;

    async fn subscribe(&self) -> Result<broadcast::Receiver<EpochManagerEvent>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::Subscribe { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;
        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_committee(&self, epoch: Epoch, shard: ShardId) -> Result<Committee<Self::Addr>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetCommittee {
                epoch,
                shard,
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_committee_within_shard_range(
        &self,
        epoch: Epoch,
        shard_range: RangeInclusive<ShardId>,
    ) -> Result<Committee<Self::Addr>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetCommitteeForShardRange {
                epoch,
                shard_range,
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_validator_node(
        &self,
        epoch: Epoch,
        addr: &Self::Addr,
    ) -> Result<ValidatorNode<Self::Addr>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetValidatorNode {
                epoch,
                addr: addr.clone(),
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_many_validator_nodes(
        &self,
        query: Vec<(Epoch, CommsPublicKey)>,
    ) -> Result<HashMap<(Epoch, Self::Addr), ValidatorNode<Self::Addr>>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetManyValidatorNodes { query, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_validator_set_merged_merkle_proof(
        &self,
        epoch: Epoch,
        validator_set: Vec<Self::Addr>,
    ) -> Result<MergedValidatorNodeMerkleProof, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetValidatorSetMergedMerkleProof {
                epoch,
                reply: tx,
                validator_set,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_our_validator_node(&self, epoch: Epoch) -> Result<ValidatorNode<Self::Addr>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetOurValidatorNode { epoch, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_local_committee_shard(&self, epoch: Epoch) -> Result<CommitteeShard, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetLocalCommitteeShard { epoch, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_committee_shard(&self, epoch: Epoch, shard: ShardId) -> Result<CommitteeShard, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetCommitteeShard {
                epoch,
                shard,
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        Ok(rx.await.map_err(|_| EpochManagerError::ReceiveError).unwrap().unwrap())
    }

    async fn current_epoch(&self) -> Result<Epoch, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::CurrentEpoch { reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn is_epoch_active(&self, epoch: Epoch) -> Result<bool, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::IsEpochValid { epoch, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_num_committees(&self, epoch: Epoch) -> Result<u32, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetNumCommittees { epoch, reply: tx })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }

    async fn get_committees_by_buckets(
        &self,
        epoch: Epoch,
        buckets: HashSet<ShardBucket>,
    ) -> Result<HashMap<ShardBucket, Committee<Self::Addr>>, EpochManagerError> {
        let (tx, rx) = oneshot::channel();
        self.tx_request
            .send(EpochManagerRequest::GetCommitteesByBuckets {
                epoch,
                buckets,
                reply: tx,
            })
            .await
            .map_err(|_| EpochManagerError::SendError)?;

        rx.await.map_err(|_| EpochManagerError::ReceiveError)?
    }
}
