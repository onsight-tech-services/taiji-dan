//   Copyright 2023 The Tari Project
//   SPDX-License-Identifier: BSD-3-Clause

use std::{
    borrow::Borrow,
    collections::HashSet,
    iter,
    ops::{DerefMut, RangeInclusive},
};

use log::*;
use serde::{Deserialize, Serialize};
use tari_common_types::types::FixedHash;
use tari_dan_common_types::{optional::Optional, Epoch, NodeAddressable, NodeHeight, ShardId};
use tari_engine_types::substate::{Substate, SubstateAddress, SubstateValue};
use tari_transaction::TransactionId;

use crate::{
    consensus_models::{Block, BlockId, QcId, QuorumCertificate},
    StateStoreReadTransaction,
    StateStoreWriteTransaction,
    StorageError,
};

const LOG_TARGET: &str = "tari::dan::storage::consensus_models::substate";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstateRecord {
    pub address: SubstateAddress,
    pub version: u32,
    pub substate_value: SubstateValue,
    pub state_hash: FixedHash,
    pub created_by_transaction: TransactionId,
    pub created_justify: QcId,
    pub created_block: BlockId,
    pub created_height: NodeHeight,
    pub created_at_epoch: Epoch,
    pub destroyed: Option<SubstateDestroyed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstateDestroyed {
    pub by_transaction: TransactionId,
    pub justify: QcId,
    pub by_block: BlockId,
    pub at_epoch: Epoch,
}

impl SubstateRecord {
    pub fn new(
        address: SubstateAddress,
        version: u32,
        substate_value: SubstateValue,
        created_at_epoch: Epoch,
        created_height: NodeHeight,
        created_block: BlockId,
        created_by_transaction: TransactionId,
        created_justify: QcId,
    ) -> Self {
        Self {
            address,
            version,
            substate_value,
            state_hash: Default::default(),
            created_height,
            created_justify,
            created_at_epoch,
            created_by_transaction,
            created_block,
            destroyed: None,
        }
    }

    pub fn to_shard_id(&self) -> ShardId {
        ShardId::from_address(&self.address, self.version)
    }

    pub fn substate_address(&self) -> &SubstateAddress {
        &self.address
    }

    pub fn substate_value(&self) -> &SubstateValue {
        &self.substate_value
    }

    pub fn into_substate_value(self) -> SubstateValue {
        self.substate_value
    }

    pub fn to_substate(&self) -> Substate {
        Substate::new(self.version, self.substate_value.clone())
    }

    pub fn into_substate(self) -> Substate {
        Substate::new(self.version, self.substate_value)
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn created_height(&self) -> NodeHeight {
        self.created_height
    }

    pub fn created_block(&self) -> BlockId {
        self.created_block
    }

    pub fn created_by_transaction(&self) -> TransactionId {
        self.created_by_transaction
    }

    pub fn created_justify(&self) -> &QcId {
        &self.created_justify
    }

    pub fn destroyed(&self) -> Option<&SubstateDestroyed> {
        self.destroyed.as_ref()
    }

    pub fn is_destroyed(&self) -> bool {
        self.destroyed.is_some()
    }
}

impl SubstateRecord {
    pub fn try_lock_all<'a, TTx: StateStoreWriteTransaction, I: IntoIterator<Item = &'a ShardId>>(
        tx: &mut TTx,
        locked_by_tx: &TransactionId,
        inputs: I,
        lock_flag: SubstateLockFlag,
    ) -> Result<SubstateLockState, StorageError> {
        tx.substates_try_lock_many(locked_by_tx, inputs, lock_flag)
    }

    pub fn check_lock_all<'a, TTx: StateStoreReadTransaction, I: IntoIterator<Item = &'a ShardId>>(
        tx: &mut TTx,
        inputs: I,
        lock_flag: SubstateLockFlag,
    ) -> Result<SubstateLockState, StorageError> {
        tx.substates_check_lock_many(inputs, lock_flag)
    }

    pub fn try_unlock_many<'a, TTx: StateStoreWriteTransaction, I: IntoIterator<Item = &'a ShardId>>(
        tx: &mut TTx,
        locked_by_tx: &TransactionId,
        inputs: I,
        lock_flag: SubstateLockFlag,
    ) -> Result<(), StorageError> {
        tx.substates_try_unlock_many(locked_by_tx, inputs, lock_flag)?;
        Ok(())
    }

    pub fn unlock_any<'a, TTx: StateStoreWriteTransaction, I: IntoIterator<Item = &'a ShardId>>(
        tx: &mut TTx,
        locked_by_tx: &TransactionId,
        inputs: I,
        lock_flag: SubstateLockFlag,
    ) -> Result<(), StorageError> {
        tx.substates_try_unlock_many(locked_by_tx, inputs, lock_flag)?;
        Ok(())
    }

    pub fn create<TTx: StateStoreWriteTransaction>(self, tx: &mut TTx) -> Result<(), StorageError> {
        tx.substates_create(self)?;
        Ok(())
    }

    pub fn exists<TTx: StateStoreReadTransaction + ?Sized>(
        tx: &mut TTx,
        shard: &ShardId,
    ) -> Result<bool, StorageError> {
        // TODO: optimise
        Ok(Self::get(tx, shard).optional()?.is_some())
    }

    pub fn any_exist<TTx: StateStoreReadTransaction + ?Sized, I: IntoIterator<Item = S>, S: Borrow<ShardId>>(
        tx: &mut TTx,
        substates: I,
    ) -> Result<bool, StorageError> {
        tx.substates_any_exist(substates)
    }

    pub fn exists_for_transaction<TTx: StateStoreReadTransaction + ?Sized>(
        tx: &mut TTx,
        transaction_id: &TransactionId,
    ) -> Result<bool, StorageError> {
        tx.substates_exists_for_transaction(transaction_id)
    }

    pub fn get<TTx: StateStoreReadTransaction + ?Sized>(
        tx: &mut TTx,
        shard: &ShardId,
    ) -> Result<SubstateRecord, StorageError> {
        tx.substates_get(shard)
    }

    pub fn get_any<'a, TTx: StateStoreReadTransaction + ?Sized, I: IntoIterator<Item = &'a ShardId>>(
        tx: &mut TTx,
        shards: I,
    ) -> Result<(Vec<SubstateRecord>, HashSet<ShardId>), StorageError> {
        let mut shards = shards.into_iter().copied().collect::<HashSet<_>>();
        let found = tx.substates_get_any(&shards)?;
        for f in &found {
            shards.remove(&f.to_shard_id());
        }

        Ok((found, shards))
    }

    pub fn get_many_within_range<TTx: StateStoreReadTransaction, B: Borrow<RangeInclusive<ShardId>>>(
        tx: &mut TTx,
        bounds: B,
        excluded_shards: &[ShardId],
    ) -> Result<Vec<SubstateRecord>, StorageError> {
        tx.substates_get_many_within_range(bounds.borrow().start(), bounds.borrow().end(), excluded_shards)
    }

    pub fn get_many_by_created_transaction<TTx: StateStoreReadTransaction>(
        tx: &mut TTx,
        transaction_id: &TransactionId,
    ) -> Result<Vec<SubstateRecord>, StorageError> {
        tx.substates_get_many_by_created_transaction(transaction_id)
    }

    pub fn get_many_by_destroyed_transaction<TTx: StateStoreReadTransaction>(
        tx: &mut TTx,
        transaction_id: &TransactionId,
    ) -> Result<Vec<SubstateRecord>, StorageError> {
        tx.substates_get_many_by_destroyed_transaction(transaction_id)
    }

    pub fn get_created_quorum_certificate<TTx: StateStoreReadTransaction>(
        &self,
        tx: &mut TTx,
    ) -> Result<QuorumCertificate<TTx::Addr>, StorageError> {
        tx.quorum_certificates_get(self.created_justify())
    }

    pub fn get_destroyed_quorum_certificate<TTx: StateStoreReadTransaction>(
        &self,
        tx: &mut TTx,
    ) -> Result<Option<QuorumCertificate<TTx::Addr>>, StorageError> {
        self.destroyed()
            .map(|destroyed| tx.quorum_certificates_get(&destroyed.justify))
            .transpose()
    }

    pub fn destroy_many<TTx: StateStoreWriteTransaction, I: IntoIterator<Item = ShardId>>(
        tx: &mut TTx,
        shard_ids: I,
        epoch: Epoch,
        destroyed_by_block: &BlockId,
        destroyed_justify: &QcId,
        destroyed_by_transaction: &TransactionId,
        require_locks: bool,
    ) -> Result<(), StorageError> {
        tx.substate_down_many(
            shard_ids,
            epoch,
            destroyed_by_block,
            destroyed_by_transaction,
            destroyed_justify,
            require_locks,
        )
    }
}

#[derive(Debug, Clone)]
pub struct SubstateCreatedProof<TAddr> {
    pub substate: SubstateData,
    pub created_qc: QuorumCertificate<TAddr>,
}

#[derive(Debug, Clone)]
pub struct SubstateData {
    pub address: SubstateAddress,
    pub version: u32,
    pub substate_value: SubstateValue,
    pub created_by_transaction: TransactionId,
}

impl From<SubstateRecord> for SubstateData {
    fn from(value: SubstateRecord) -> Self {
        Self {
            address: value.address,
            version: value.version,
            substate_value: value.substate_value,
            created_by_transaction: value.created_by_transaction,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SubstateUpdate<TAddr> {
    Create(SubstateCreatedProof<TAddr>),
    Destroy {
        shard_id: ShardId,
        proof: QuorumCertificate<TAddr>,
        destroyed_by_transaction: TransactionId,
    },
}

impl<TAddr> SubstateUpdate<TAddr> {
    pub fn is_create(&self) -> bool {
        matches!(self, Self::Create(_))
    }

    pub fn is_destroy(&self) -> bool {
        matches!(self, Self::Destroy { .. })
    }
}

impl<TAddr: NodeAddressable> SubstateUpdate<TAddr> {
    pub fn apply<TTx>(self, tx: &mut TTx, block: &Block<TAddr>) -> Result<(), StorageError>
    where
        TTx: StateStoreWriteTransaction<Addr = TAddr> + DerefMut,
        TTx::Target: StateStoreReadTransaction,
    {
        match self {
            Self::Create(proof) => {
                debug!(
                    target: LOG_TARGET,
                    "🌲 Applying substate CREATE for {} v{}",
                    proof.substate.address, proof.substate.version
                );
                proof.created_qc.save(tx)?;
                SubstateRecord {
                    address: proof.substate.address,
                    version: proof.substate.version,
                    substate_value: proof.substate.substate_value,
                    state_hash: Default::default(),
                    created_by_transaction: proof.substate.created_by_transaction,
                    created_justify: *proof.created_qc.id(),
                    created_block: *block.id(),
                    created_height: block.height(),
                    created_at_epoch: block.epoch(),
                    destroyed: None,
                }
                .create(tx)?;
            },
            Self::Destroy {
                shard_id,
                proof,
                destroyed_by_transaction,
            } => {
                debug!(
                    target: LOG_TARGET,
                    "🔥 Applying substate DESTROY for shard {} (transaction {})",
                    shard_id,
                    destroyed_by_transaction
                );
                proof.save(tx)?;
                SubstateRecord::destroy_many(
                    tx,
                    iter::once(shard_id),
                    block.epoch(),
                    block.id(),
                    proof.id(),
                    &destroyed_by_transaction,
                    false,
                )?;
            },
        }

        Ok(())
    }
}

impl<TAddr> From<SubstateCreatedProof<TAddr>> for SubstateUpdate<TAddr> {
    fn from(value: SubstateCreatedProof<TAddr>) -> Self {
        Self::Create(value)
    }
}

/// Substate lock flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SubstateLockFlag {
    Read = 0x01,
    Write = 0x02,
}

#[derive(Debug, Clone, Copy)]
pub enum SubstateLockState {
    /// The lock was successfully acquired
    LockAcquired,
    /// The lock was not acquired because some substates are DOWN
    SomeDestroyed,
    /// Some substates are locked for write
    SomeAlreadyWriteLocked,
    /// Some outputs substates exist. This indicates that that we attempted to lock an output but the output is already
    /// a substate (Up or DOWN)
    SomeOutputSubstatesExist,
    /// Some inputs substates do not exist
    InputsConfict,
}

impl SubstateLockState {
    pub fn is_acquired(&self) -> bool {
        matches!(self, Self::LockAcquired)
    }
}
