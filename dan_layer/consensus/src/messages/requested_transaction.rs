//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use serde::Serialize;
use taiji_dan_common_types::Epoch;
use taiji_dan_storage::consensus_models::BlockId;
use taiji_transaction::Transaction;

#[derive(Debug, Clone, Serialize)]
pub struct RequestedTransactionMessage {
    pub epoch: Epoch,
    pub block_id: BlockId,
    pub transactions: Vec<Transaction>,
}
