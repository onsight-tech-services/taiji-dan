//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use taiji_consensus::traits::StateManager;
use taiji_dan_common_types::ShardId;
use taiji_dan_storage::{
    consensus_models::{Block, ExecutedTransaction, SubstateRecord},
    StateStore,
    StorageError,
};

pub struct TaijiStateManager;

impl TaijiStateManager {
    pub fn new() -> Self {
        Self
    }
}

impl<TStateStore: StateStore> StateManager<TStateStore> for TaijiStateManager {
    type Error = StorageError;

    fn commit_transaction(
        &self,
        tx: &mut TStateStore::WriteTransaction<'_>,
        block: &Block<TStateStore::Addr>,
        transaction: &ExecutedTransaction,
    ) -> Result<(), Self::Error> {
        let Some(diff) = transaction.result().finalize.result.accept() else {
            // We should only commit accepted transactions, might want to change this API to reflect that
            return Ok(());
        };

        let down_shards = diff
            .down_iter()
            .map(|(addr, version)| ShardId::from_address(addr, *version));
        SubstateRecord::destroy_many(
            tx,
            down_shards,
            block.epoch(),
            block.id(),
            block.justify().id(),
            transaction.id(),
            true,
        )?;

        let to_up = diff.up_iter().map(|(addr, substate)| {
            SubstateRecord::new(
                addr.clone(),
                substate.version(),
                substate.substate_value().clone(),
                block.epoch(),
                block.height(),
                *block.id(),
                *transaction.id(),
                *block.justify().id(),
            )
        });

        for up in to_up {
            up.create(tx)?;
        }

        Ok(())
    }
}