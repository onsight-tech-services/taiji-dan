//    Copyright 2023 OnSight Tech Services LLC
//    SPDX-License-Identifier: BSD-3-Clause

use taiji_dan_common_types::NodeHeight;
use taiji_dan_storage::consensus_models::BlockId;

#[derive(Debug, Clone)]
pub enum HotstuffEvent {
    /// A block has been committed
    BlockCommitted { block_id: BlockId, height: NodeHeight },
    /// A critical failure occurred in consensus
    Failure { message: String },
    /// A leader has timed out
    LeaderTimeout { new_height: NodeHeight },
}
