//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use taiji_comms::protocol::rpc::RpcError;
use taiji_consensus::hotstuff::HotStuffError;
use taiji_dan_storage::{
    consensus_models::{BlockId, TransactionPoolError},
    StorageError,
};
use taiji_epoch_manager::EpochManagerError;
use taiji_validator_node_rpc::ValidatorNodeRpcClientError;

#[derive(Debug, thiserror::Error)]
pub enum CommsRpcConsensusSyncError {
    #[error("Epoch manager error: {0}")]
    EpochManagerError(#[from] EpochManagerError),
    #[error("RPC error: {0}")]
    RpcError(#[from] RpcError),
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    #[error("Validator node client error: {0}")]
    ValidatorNodeClientError(#[from] ValidatorNodeRpcClientError),
    #[error("Transaction pool error: {0}")]
    TransactionPoolError(#[from] TransactionPoolError),
    #[error("Invalid response: {0}")]
    InvalidResponse(anyhow::Error),
    #[error("Block {block_id} failed SafeNode predicate")]
    BlockNotSafe { block_id: BlockId },
    #[error("No peers available. The committee size is {committee_size}")]
    NoPeersAvailable { committee_size: usize },
}

impl From<CommsRpcConsensusSyncError> for HotStuffError {
    fn from(value: CommsRpcConsensusSyncError) -> Self {
        HotStuffError::SyncError(value.into())
    }
}
