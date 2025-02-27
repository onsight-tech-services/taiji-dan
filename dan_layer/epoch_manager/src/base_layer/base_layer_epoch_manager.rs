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

use std::{
    cmp,
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
};

use log::*;
use tari_base_node_client::{grpc::GrpcBaseNodeClient, types::BaseLayerConsensusConstants, BaseNodeClient};
use tari_common_types::types::{FixedHash, PublicKey};
use tari_comms::types::CommsPublicKey;
use tari_core::{blocks::BlockHeader, transactions::transaction_components::ValidatorNodeRegistration};
use tari_crypto::tari_utilities::ByteArray;
use tari_dan_common_types::{
    committee::{Committee, CommitteeShard},
    hashing::{MergedValidatorNodeMerkleProof, ValidatorNodeBalancedMerkleTree, ValidatorNodeMerkleProof},
    optional::Optional,
    shard_bucket::ShardBucket,
    Epoch,
    ShardId,
};
use tari_dan_storage::global::{models::ValidatorNode, DbEpoch, GlobalDb, MetadataKey};
use tari_dan_storage_sqlite::global::SqliteGlobalDbAdapter;
use tari_mmr::MergedBalancedBinaryMerkleProof;
use tokio::sync::broadcast;

use crate::{base_layer::config::EpochManagerConfig, error::EpochManagerError, EpochManagerEvent};

const LOG_TARGET: &str = "tari::dan::epoch_manager::base_layer";

#[derive(Clone)]
pub struct BaseLayerEpochManager<TGlobalStore, TBaseNodeClient> {
    global_db: GlobalDb<TGlobalStore>,
    base_node_client: TBaseNodeClient,
    config: EpochManagerConfig,
    current_epoch: Epoch,
    current_block_height: u64,
    tx_events: broadcast::Sender<EpochManagerEvent>,
    node_public_key: CommsPublicKey,
    current_shard_key: Option<ShardId>,
    base_layer_consensus_constants: Option<BaseLayerConsensusConstants>,
    is_initial_base_layer_sync_complete: bool,
}

impl BaseLayerEpochManager<SqliteGlobalDbAdapter, GrpcBaseNodeClient> {
    pub fn new(
        config: EpochManagerConfig,
        global_db: GlobalDb<SqliteGlobalDbAdapter>,
        base_node_client: GrpcBaseNodeClient,
        tx_events: broadcast::Sender<EpochManagerEvent>,
        node_public_key: CommsPublicKey,
    ) -> Self {
        Self {
            global_db,
            base_node_client,
            config,
            current_epoch: Epoch(0),
            current_block_height: 0,
            tx_events,
            node_public_key,
            current_shard_key: None,
            base_layer_consensus_constants: None,
            is_initial_base_layer_sync_complete: false,
        }
    }

    pub async fn load_initial_state(&mut self) -> Result<(), EpochManagerError> {
        self.refresh_base_layer_consensus_constants().await?;

        let mut tx = self.global_db.create_transaction()?;
        let mut metadata = self.global_db.metadata(&mut tx);
        self.current_epoch = metadata
            .get_metadata(MetadataKey::EpochManagerCurrentEpoch)?
            .unwrap_or(Epoch(0));
        self.current_shard_key = metadata.get_metadata(MetadataKey::EpochManagerCurrentShardKey)?;
        self.current_block_height = metadata
            .get_metadata(MetadataKey::EpochManagerCurrentBlockHeight)?
            .unwrap_or(0);

        Ok(())
    }

    pub async fn update_epoch(&mut self, block_height: u64, block_hash: FixedHash) -> Result<(), EpochManagerError> {
        let base_layer_constants = self.base_node_client.get_consensus_constants(block_height).await?;
        let epoch = base_layer_constants.height_to_epoch(block_height);
        self.update_current_block_height(block_height)?;
        if self.current_epoch >= epoch {
            // no need to update the epoch
            return Ok(());
        }

        info!(target: LOG_TARGET, "🌟 A new epoch {} is upon us", epoch);
        // extract and store in database the MMR of the epoch's validator nodes
        let epoch_header = self.base_node_client.get_header_by_hash(block_hash).await?;

        // persist the epoch data including the validator node set
        self.insert_current_epoch(epoch, epoch_header)?;
        self.update_base_layer_consensus_constants(base_layer_constants)?;
        self.assign_validators_for_epoch()?;

        // Only publish an epoch change event if we have synced the base layer (see on_scanning_complete)
        if self.is_initial_base_layer_sync_complete {
            self.publish_event(EpochManagerEvent::EpochChanged(epoch));
        }

        Ok(())
    }

    fn assign_validators_for_epoch(&mut self) -> Result<(), EpochManagerError> {
        let (start_epoch, end_epoch) = self.get_epoch_range(self.current_epoch)?;
        let mut tx = self.global_db.create_transaction()?;
        let mut validator_nodes = self.global_db.validator_nodes(&mut tx);

        let vns = validator_nodes.get_all_within_epochs(start_epoch, end_epoch)?;

        let num_committees = calculate_num_committees(vns.len() as u64, self.config.committee_size);

        for vn in &vns {
            validator_nodes.set_committee_bucket(vn.shard_key, vn.shard_key.to_committee_bucket(num_committees))?;
        }
        tx.commit()?;

        if let Some(vn) = vns.iter().find(|vn| vn.address == self.node_public_key) {
            self.publish_event(EpochManagerEvent::ThisValidatorIsRegistered {
                epoch: self.current_epoch,
                shard_key: vn.shard_key,
            });
        }

        Ok(())
    }

    pub async fn get_base_layer_consensus_constants(
        &mut self,
    ) -> Result<&BaseLayerConsensusConstants, EpochManagerError> {
        if let Some(ref constants) = self.base_layer_consensus_constants {
            return Ok(constants);
        }

        self.refresh_base_layer_consensus_constants().await?;

        Ok(self
            .base_layer_consensus_constants
            .as_ref()
            .expect("update_base_layer_consensus_constants did not set constants"))
    }

    async fn refresh_base_layer_consensus_constants(&mut self) -> Result<(), EpochManagerError> {
        let tip = self.base_node_client.get_tip_info().await?;
        let dan_tip = tip
            .height_of_longest_chain
            .saturating_sub(self.config.base_layer_confirmations);

        let constants = self.base_node_client.get_consensus_constants(dan_tip).await?;
        self.update_base_layer_consensus_constants(constants)?;
        Ok(())
    }

    pub async fn add_validator_node_registration(
        &mut self,
        block_height: u64,
        registration: ValidatorNodeRegistration,
    ) -> Result<(), EpochManagerError> {
        let constants = self.get_base_layer_consensus_constants().await?;
        let next_epoch = constants.height_to_epoch(block_height) + Epoch(1);
        let next_epoch_height = constants.epoch_to_height(next_epoch);

        let shard_key = self
            .base_node_client
            .get_shard_key(next_epoch_height, registration.public_key())
            .await?
            .ok_or_else(|| EpochManagerError::ShardKeyNotFound {
                public_key: registration.public_key().clone(),
                block_height,
            })?;

        let mut tx = self.global_db.create_transaction()?;
        self.global_db.validator_nodes(&mut tx).insert_validator_node(
            registration.public_key().clone(),
            shard_key,
            next_epoch,
            registration.claim_public_key().clone(),
        )?;

        if *registration.public_key() == self.node_public_key {
            let mut metadata = self.global_db.metadata(&mut tx);
            metadata.set_metadata(MetadataKey::EpochManagerCurrentShardKey, &shard_key)?;
            let last_registration_epoch = metadata
                .get_metadata::<Epoch>(MetadataKey::EpochManagerLastEpochRegistration)?
                .unwrap_or(Epoch(0));
            if last_registration_epoch < next_epoch {
                metadata.set_metadata(MetadataKey::EpochManagerLastEpochRegistration, &next_epoch)?;
            }
            self.current_shard_key = Some(shard_key);
            info!(
                target: LOG_TARGET,
                "📋️ This validator node is registered for epoch {}, shard key: {} ", next_epoch, shard_key
            );
        }

        tx.commit()?;

        Ok(())
    }

    fn insert_current_epoch(&mut self, epoch: Epoch, header: BlockHeader) -> Result<(), EpochManagerError> {
        let epoch_height = epoch.0;
        let db_epoch = DbEpoch {
            epoch: epoch_height,
            validator_node_mr: header.validator_node_mr.to_vec(),
        };

        let mut tx = self.global_db.create_transaction()?;

        self.global_db.epochs(&mut tx).insert_epoch(db_epoch)?;
        self.global_db
            .metadata(&mut tx)
            .set_metadata(MetadataKey::EpochManagerCurrentEpoch, &epoch)?;

        tx.commit()?;
        self.current_epoch = epoch;
        Ok(())
    }

    fn update_base_layer_consensus_constants(
        &mut self,
        base_layer_constants: BaseLayerConsensusConstants,
    ) -> Result<(), EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        self.global_db
            .metadata(&mut tx)
            .set_metadata(MetadataKey::BaseLayerConsensusConstants, &base_layer_constants)?;
        tx.commit()?;
        self.base_layer_consensus_constants = Some(base_layer_constants);
        Ok(())
    }

    fn update_current_block_height(&mut self, block_height: u64) -> Result<(), EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        self.global_db
            .metadata(&mut tx)
            .set_metadata(MetadataKey::EpochManagerCurrentBlockHeight, &block_height)?;
        tx.commit()?;
        self.current_block_height = block_height;
        Ok(())
    }

    pub fn current_epoch(&self) -> Epoch {
        self.current_epoch
    }

    pub fn current_block_height(&self) -> u64 {
        self.current_block_height
    }

    pub fn get_validator_node(
        &self,
        epoch: Epoch,
        public_key: &CommsPublicKey,
    ) -> Result<Option<ValidatorNode<CommsPublicKey>>, EpochManagerError> {
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
        debug!(
            target: LOG_TARGET,
            "get_validator_node: epoch {}-{} with public key {}", start_epoch, end_epoch, public_key,
        );
        let mut tx = self.global_db.create_transaction()?;
        let vn = self
            .global_db
            .validator_nodes(&mut tx)
            .get(start_epoch, end_epoch, ByteArray::as_bytes(public_key))
            .optional()?;

        Ok(vn)
    }

    pub fn get_many_validator_nodes(
        &self,
        epoch_validators: Vec<(Epoch, CommsPublicKey)>,
    ) -> Result<HashMap<(Epoch, CommsPublicKey), ValidatorNode<CommsPublicKey>>, EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        #[allow(clippy::mutable_key_type)]
        let mut validators = HashMap::with_capacity(epoch_validators.len());

        for (epoch, public_key) in epoch_validators {
            let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
            let vn = self
                .global_db
                .validator_nodes(&mut tx)
                .get(start_epoch, end_epoch, ByteArray::as_bytes(&public_key))
                .optional()?
                .ok_or_else(|| EpochManagerError::ValidatorNodeNotRegistered {
                    address: public_key.to_string(),
                    epoch,
                })?;

            validators.insert((epoch, public_key), vn);
        }

        Ok(validators)
    }

    pub fn last_registration_epoch(&self) -> Result<Option<Epoch>, EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        let mut metadata = self.global_db.metadata(&mut tx);
        let last_registration_epoch = metadata.get_metadata(MetadataKey::EpochManagerLastEpochRegistration)?;
        Ok(last_registration_epoch)
    }

    pub fn update_last_registration_epoch(&self, epoch: Epoch) -> Result<(), EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        self.global_db
            .metadata(&mut tx)
            .set_metadata(MetadataKey::EpochManagerLastEpochRegistration, &epoch)?;
        tx.commit()?;
        Ok(())
    }

    pub fn is_epoch_valid(&self, epoch: Epoch) -> bool {
        let current_epoch = self.current_epoch();
        // Allow for 10 epochs behind. TODO: Properly define a "valid" epoch
        epoch.as_u64() >= current_epoch.as_u64().saturating_sub(10) && epoch.as_u64() <= current_epoch.as_u64()
    }

    pub fn get_committees(
        &self,
        epoch: Epoch,
        shards: &HashSet<ShardId>,
    ) -> Result<HashMap<ShardBucket, Committee<CommsPublicKey>>, EpochManagerError> {
        let num_committees = self.get_number_of_committees(epoch)?;
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
        let mut tx = self.global_db.create_transaction()?;
        let mut validator_node_db = self.global_db.validator_nodes(&mut tx);
        let buckets = shards
            .iter()
            .map(|shard| shard.to_committee_bucket(num_committees))
            .collect();
        let result = validator_node_db.get_committees_by_buckets(start_epoch, end_epoch, buckets)?;
        Ok(result)
    }

    pub fn get_committee_vns_from_shard_key(
        &self,
        epoch: Epoch,
        shard: ShardId,
    ) -> Result<Vec<ValidatorNode<CommsPublicKey>>, EpochManagerError> {
        // retrieve the validator nodes for this epoch from database, sorted by shard_key
        let vns = self.get_validator_nodes_per_epoch(epoch)?;
        if vns.is_empty() {
            return Err(EpochManagerError::NoCommitteeVns { shard_id: shard, epoch });
        }

        let num_committees = calculate_num_committees(vns.len() as u64, self.config.committee_size);
        if num_committees == 1 {
            return Ok(vns);
        }

        // A shard bucket is a equal slice of the shard space that a validator fits into
        let shard_bucket = shard.to_committee_bucket(num_committees);

        let selected_vns = vns
            .into_iter()
            .filter(|vn| {
                vn.committee_bucket
                    .unwrap_or_else(|| vn.shard_key.to_committee_bucket(num_committees)) ==
                    shard_bucket
            })
            .collect();

        Ok(selected_vns)
    }

    pub fn get_committee(&self, epoch: Epoch, shard: ShardId) -> Result<Committee<CommsPublicKey>, EpochManagerError> {
        let result = self.get_committee_vns_from_shard_key(epoch, shard)?;
        Ok(Committee::new(result.into_iter().map(|v| v.address).collect()))
    }

    pub fn is_validator_in_committee(
        &self,
        epoch: Epoch,
        shard: ShardId,
        identity: &CommsPublicKey,
    ) -> Result<bool, EpochManagerError> {
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
        let mut tx = self.global_db.create_transaction()?;
        let mut vn_db = self.global_db.validator_nodes(&mut tx);
        let num_vns = vn_db.count(start_epoch, end_epoch)?;
        let vn = vn_db.get(start_epoch, end_epoch, ByteArray::as_bytes(identity))?;
        let num_committees = calculate_num_committees(num_vns, self.config.committee_size);
        let shard_bucket = shard.to_committee_bucket(num_committees);
        match vn.committee_bucket {
            Some(bucket) => Ok(bucket == shard_bucket),
            None => Ok(false),
        }
    }

    pub fn get_number_of_committees(&self, epoch: Epoch) -> Result<u32, EpochManagerError> {
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;

        let mut tx = self.global_db.create_transaction()?;
        let num_vns = self.global_db.validator_nodes(&mut tx).count(start_epoch, end_epoch)?;
        Ok(calculate_num_committees(num_vns, self.config.committee_size))
    }

    fn get_epoch_range(&self, end_epoch: Epoch) -> Result<(Epoch, Epoch), EpochManagerError> {
        let consensus_constants = self
            .base_layer_consensus_constants
            .as_ref()
            .ok_or(EpochManagerError::BaseLayerConsensusConstantsNotSet)?;

        let start_epoch = end_epoch.saturating_sub(consensus_constants.validator_node_registration_expiry());
        Ok((start_epoch, end_epoch))
    }

    pub fn get_validator_nodes_per_epoch(
        &self,
        epoch: Epoch,
    ) -> Result<Vec<ValidatorNode<CommsPublicKey>>, EpochManagerError> {
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;

        let mut tx = self.global_db.create_transaction()?;
        let db_vns = self
            .global_db
            .validator_nodes(&mut tx)
            .get_all_within_epochs(start_epoch, end_epoch)?;
        let vns = db_vns.into_iter().map(Into::into).collect();
        Ok(vns)
    }

    pub fn get_validator_node_merkle_root(&self, epoch: Epoch) -> Result<Vec<u8>, EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;

        let query_res = self.global_db.epochs(&mut tx).get_epoch_data(epoch.0)?;

        match query_res {
            Some(db_epoch) => Ok(db_epoch.validator_node_mr),
            None => Err(EpochManagerError::NoEpochFound(epoch)),
        }
    }

    fn get_validator_node_balanced_merkle_tree(
        &self,
        epoch: Epoch,
    ) -> Result<ValidatorNodeBalancedMerkleTree, EpochManagerError> {
        let db = {
            let mut tx = self.global_db.create_transaction()?;
            self.global_db.bmt(&mut tx).get_bmt(epoch)
        }?;
        if let Some(bmt) = db {
            return Ok(bmt);
        }
        let vns = self.get_validator_nodes_per_epoch(epoch)?;

        let vn_bmt_leaves = vns.iter().map(|vn| vn.node_hash().to_vec()).collect();
        let vn_bmt = ValidatorNodeBalancedMerkleTree::create(vn_bmt_leaves);
        let mut tx = self.global_db.create_transaction()?;
        self.global_db.bmt(&mut tx).insert_bmt(epoch.as_u64(), vn_bmt.clone())?;
        tx.commit()?;
        Ok(vn_bmt)
    }

    pub fn get_validator_set_merged_merkle_proof(
        &self,
        epoch: Epoch,
        validators: Vec<CommsPublicKey>,
    ) -> Result<MergedValidatorNodeMerkleProof, EpochManagerError> {
        let mut proofs = Vec::with_capacity(validators.len());
        let bmt = self.get_validator_node_balanced_merkle_tree(epoch)?;

        for validator in &validators {
            let vn = self.get_validator_node(epoch, validator)?.ok_or_else(|| {
                EpochManagerError::ValidatorNodeNotRegistered {
                    address: self.node_public_key.to_string(),
                    epoch,
                }
            })?;
            let leaf_index = bmt.find_leaf_index_for_hash(&vn.node_hash().to_vec())?;

            let proof = ValidatorNodeMerkleProof::generate_proof(&bmt, leaf_index as usize)?;
            proofs.push(proof);
        }

        Ok(MergedBalancedBinaryMerkleProof::create_from_proofs(&proofs).unwrap())
    }

    pub async fn on_scanning_complete(&mut self) -> Result<(), EpochManagerError> {
        self.refresh_base_layer_consensus_constants().await?;

        if !self.is_initial_base_layer_sync_complete {
            info!(
                target: LOG_TARGET,
                "🌟 Initial base layer sync complete. Current epoch is {}", self.current_epoch
            );
            self.publish_event(EpochManagerEvent::EpochChanged(self.current_epoch));
            self.is_initial_base_layer_sync_complete = true;
        }

        Ok(())
    }

    pub async fn remaining_registration_epochs(&mut self) -> Result<Option<Epoch>, EpochManagerError> {
        let last_registration_epoch = match self.last_registration_epoch()? {
            Some(epoch) => epoch,
            None => return Ok(None),
        };

        let constants = self.get_base_layer_consensus_constants().await?;
        let expiry = constants.validator_node_registration_expiry();

        // Note this can be negative in some cases
        let num_blocks_since_last_reg = self.current_epoch.saturating_sub(last_registration_epoch);

        // None indicates that we are not registered, or a previous registration has expired
        Ok(expiry.checked_sub(num_blocks_since_last_reg))
    }

    pub fn get_local_shard_range(
        &self,
        epoch: Epoch,
        addr: &CommsPublicKey,
    ) -> Result<RangeInclusive<ShardId>, EpochManagerError> {
        let vn =
            self.get_validator_node(epoch, addr)?
                .ok_or_else(|| EpochManagerError::ValidatorNodeNotRegistered {
                    address: addr.to_string(),
                    epoch,
                })?;

        let num_committees = self.get_number_of_committees(epoch)?;
        debug!(
            target: LOG_TARGET,
            "VN {} epoch: {}, num_committees: {}", addr, epoch, num_committees
        );
        Ok(vn.shard_key.to_committee_range(num_committees))
    }

    pub fn get_committee_for_shard_range(
        &self,
        epoch: Epoch,
        shard_range: RangeInclusive<ShardId>,
    ) -> Result<Committee<CommsPublicKey>, EpochManagerError> {
        let num_committees = self.get_number_of_committees(epoch)?;

        // Since we have fixed boundaries for committees, we want to include all validators within any range "touching"
        // the range we are searching for. For e.g. the committee for half a committee shard is the same committee as
        // for a whole committee shard.
        let rounded_shard_range = {
            let start_range = shard_range.start().to_committee_range(num_committees);
            let end_range = shard_range.end().to_committee_range(num_committees);
            *start_range.start()..=*end_range.end()
        };
        let mut tx = self.global_db.create_transaction()?;
        let mut validator_node_db = self.global_db.validator_nodes(&mut tx);
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
        let validators = validator_node_db.get_by_shard_range(start_epoch, end_epoch, rounded_shard_range)?;
        Ok(Committee::new(validators.into_iter().map(|v| v.address).collect()))
    }

    pub fn get_our_validator_node(&self, epoch: Epoch) -> Result<ValidatorNode<CommsPublicKey>, EpochManagerError> {
        let vn = self.get_validator_node(epoch, &self.node_public_key)?.ok_or_else(|| {
            EpochManagerError::ValidatorNodeNotRegistered {
                address: self.node_public_key.to_string(),
                epoch,
            }
        })?;
        Ok(vn)
    }

    pub fn get_total_validator_count(&self, epoch: Epoch) -> Result<u64, EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        let mut validator_node_db = self.global_db.validator_nodes(&mut tx);
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
        let num_validators = validator_node_db.count(start_epoch, end_epoch)?;
        Ok(num_validators)
    }

    pub fn get_num_committees(&self, epoch: Epoch) -> Result<u32, EpochManagerError> {
        let total_vns = self.get_total_validator_count(epoch)?;
        let committee_size = self.config.committee_size;
        let num_committees = calculate_num_committees(total_vns, committee_size);
        Ok(num_committees)
    }

    pub fn get_committee_shard(&self, epoch: Epoch, shard: ShardId) -> Result<CommitteeShard, EpochManagerError> {
        let num_committees = self.get_number_of_committees(epoch)?;
        let bucket = shard.to_committee_bucket(num_committees);
        let mut tx = self.global_db.create_transaction()?;
        let mut validator_node_db = self.global_db.validator_nodes(&mut tx);
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
        let num_validators = validator_node_db.count_in_bucket(start_epoch, end_epoch, bucket)?;
        let num_validators = u32::try_from(num_validators).map_err(|_| EpochManagerError::IntegerOverflow {
            func: "get_committee_shard",
        })?;
        Ok(CommitteeShard::new(num_committees, num_validators, bucket))
    }

    pub fn get_local_committee_shard(&self, epoch: Epoch) -> Result<CommitteeShard, EpochManagerError> {
        let vn = self.get_validator_node(epoch, &self.node_public_key)?.ok_or_else(|| {
            EpochManagerError::ValidatorNodeNotRegistered {
                address: self.node_public_key.to_string(),
                epoch,
            }
        })?;
        self.get_committee_shard(epoch, vn.shard_key)
    }

    pub fn get_committees_by_buckets(
        &self,
        epoch: Epoch,
        buckets: HashSet<ShardBucket>,
    ) -> Result<HashMap<ShardBucket, Committee<CommsPublicKey>>, EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        let mut validator_node_db = self.global_db.validator_nodes(&mut tx);
        let (start_epoch, end_epoch) = self.get_epoch_range(epoch)?;
        let committees = validator_node_db.get_committees_by_buckets(start_epoch, end_epoch, buckets)?;
        Ok(committees)
    }

    pub fn get_fee_claim_public_key(&self) -> Result<Option<PublicKey>, EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        let mut metadata = self.global_db.metadata(&mut tx);
        let fee_claim_public_key = metadata.get_metadata(MetadataKey::EpochManagerFeeClaimPublicKey)?;
        Ok(fee_claim_public_key)
    }

    pub fn set_fee_claim_public_key(&mut self, public_key: CommsPublicKey) -> Result<(), EpochManagerError> {
        let mut tx = self.global_db.create_transaction()?;
        let mut metadata = self.global_db.metadata(&mut tx);
        metadata.set_metadata(MetadataKey::EpochManagerFeeClaimPublicKey, &public_key)?;
        tx.commit()?;
        Ok(())
    }

    fn publish_event(&mut self, event: EpochManagerEvent) {
        let _ignore = self.tx_events.send(event);
    }
}

fn calculate_num_committees(num_vns: u64, committee_size: u32) -> u32 {
    // Number of committees is proportional to the number of validators available.
    // We cap the number of committees to u32::MAX (for a committee_size of 10 that's over 42 billion validators)
    cmp::min(cmp::max(1, num_vns / u64::from(committee_size)), u64::from(u32::MAX)) as u32
}
