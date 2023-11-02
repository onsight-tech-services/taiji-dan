//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use taiji_dan_storage::{
    consensus_models::{Block, ExecutedTransaction},
    StateStore,
};

pub trait StateManager<TStateStore: StateStore> {
    type Error: std::error::Error + Send + Sync + 'static;

    fn commit_transaction(
        &self,
        tx: &mut TStateStore::WriteTransaction<'_>,
        block: &Block<TStateStore::Addr>,
        transaction: &ExecutedTransaction,
    ) -> Result<(), Self::Error>;
}
