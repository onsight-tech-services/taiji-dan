//    Copyright 2023 The Tari Project
//    SPDX-License-Identifier: BSD-3-Clause

use tari_dan_app_utilities::{
    template_manager::interface::TemplateManagerError,
    transaction_executor::TransactionProcessorError,
};
use tari_dan_common_types::Epoch;
use tari_dan_storage::{consensus_models::TransactionPoolError, StorageError};
use tari_epoch_manager::EpochManagerError;
use tari_transaction::TransactionId;
use tokio::sync::{mpsc, oneshot};

use crate::{
    dry_run_transaction_processor::DryRunTransactionProcessorError,
    p2p::services::{mempool::MempoolRequest, messaging::MessagingError},
    substate_resolver::SubstateResolverError,
    virtual_substate::VirtualSubstateError,
};

#[derive(thiserror::Error, Debug)]
pub enum MempoolError {
    #[error("Epoch Manager Error: {0}")]
    EpochManagerError(#[from] EpochManagerError),
    #[error("Broadcast failed: {0}")]
    BroadcastFailed(#[from] MessagingError),
    #[error("Internal service request cancelled")]
    RequestCancelled,
    #[error("DryRunTransactionProcessor Error: {0}")]
    DryRunTransactionProcessorError(#[from] DryRunTransactionProcessorError),
    #[error("Execution thread failure: {0}")]
    ExecutionThreadFailure(String),
    #[error("SubstateResolver Error: {0}")]
    SubstateResolverError(#[from] SubstateResolverError),
    #[error("Transaction Execution Error: {0}")]
    TransactionExecutionError(#[from] TransactionProcessorError),
    #[error("Storage Error: {0}")]
    StorageError(#[from] StorageError),
    #[error("Virtual substate error: {0}")]
    VirtualSubstateError(#[from] VirtualSubstateError),
    #[error("Transaction pool error: {0}")]
    TransactionPoolError(#[from] TransactionPoolError),

    // TODO: move these to MempoolValidationError type
    #[error("Invalid template address: {0}")]
    InvalidTemplateAddress(#[from] TemplateManagerError),
    #[error("No fee instructions")]
    NoFeeInstructions,
    #[error("Input refs downed")]
    InputRefsDowned,
    #[error("Output substate exists in transaction {transaction_id}")]
    OutputSubstateExists { transaction_id: TransactionId },
    #[error("Validator fee claim instruction in transaction {transaction_id} contained invalid epoch {given_epoch}")]
    ValidatorFeeClaimEpochInvalid {
        transaction_id: TransactionId,
        given_epoch: Epoch,
    },
    #[error("Current epoch ({current_epoch}) is less than minimum epoch ({min_epoch}) required for transaction")]
    CurrentEpochLessThanMinimum { current_epoch: Epoch, min_epoch: Epoch },
    #[error("Current epoch ({current_epoch}) is greater than maximum epoch ({max_epoch}) required for transaction")]
    CurrentEpochGreaterThanMaximum { current_epoch: Epoch, max_epoch: Epoch },
    #[error("Transaction {transaction_id} does not have any inputs")]
    NoInputs { transaction_id: TransactionId },
    #[error("Executed transaction {transaction_id} does not involved any shards")]
    NoInvolvedShards { transaction_id: TransactionId },
}

impl From<mpsc::error::SendError<MempoolRequest>> for MempoolError {
    fn from(_: mpsc::error::SendError<MempoolRequest>) -> Self {
        Self::RequestCancelled
    }
}

impl From<oneshot::error::RecvError> for MempoolError {
    fn from(_: oneshot::error::RecvError) -> Self {
        Self::RequestCancelled
    }
}
