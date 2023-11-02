//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use taiji_base_node_client::BaseNodeClientError;
use taiji_dan_storage_sqlite::error::SqliteStorageError;
use taiji_mmr::{BalancedBinaryMerkleProofError, BalancedBinaryMerkleTreeError};

use crate::EpochManagerError;

impl From<BaseNodeClientError> for EpochManagerError {
    fn from(e: BaseNodeClientError) -> Self {
        Self::BaseNodeError(anyhow::Error::from(e))
    }
}

impl From<BalancedBinaryMerkleProofError> for EpochManagerError {
    fn from(e: BalancedBinaryMerkleProofError) -> Self {
        Self::BalancedBinaryMerkleProofError(anyhow::Error::from(e))
    }
}

impl From<BalancedBinaryMerkleTreeError> for EpochManagerError {
    fn from(e: BalancedBinaryMerkleTreeError) -> Self {
        Self::BalancedBinaryMerkleTreeError(anyhow::Error::from(e))
    }
}

impl From<SqliteStorageError> for EpochManagerError {
    fn from(e: SqliteStorageError) -> Self {
        Self::SqlLiteStorageError(anyhow::Error::from(e))
    }
}
