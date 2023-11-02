//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use async_trait::async_trait;
use taiji_consensus::{
    hotstuff::HotStuffError,
    traits::{SyncManager, SyncStatus},
};

#[derive(Clone)]
pub struct AlwaysSyncedSyncManager;

#[async_trait]
impl SyncManager for AlwaysSyncedSyncManager {
    type Error = HotStuffError;

    async fn check_sync(&self) -> Result<SyncStatus, Self::Error> {
        Ok(SyncStatus::UpToDate)
    }

    async fn sync(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}
