//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use async_trait::async_trait;
use taiji_dan_common_types::Epoch;
use taiji_dan_engine::{runtime::VirtualSubstates, state_store::memory::MemoryStateStore};
use taiji_transaction::Transaction;

#[async_trait]
pub trait SubstateResolver {
    type Error: Send + Sync + 'static;

    async fn resolve(&self, transaction: &Transaction, out: &MemoryStateStore) -> Result<(), Self::Error>;

    async fn resolve_virtual_substates(
        &self,
        transaction: &Transaction,
        current_epoch: Epoch,
    ) -> Result<VirtualSubstates, Self::Error>;
}
