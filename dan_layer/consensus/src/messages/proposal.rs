//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use serde::Serialize;
use taiji_dan_storage::consensus_models::Block;

#[derive(Debug, Clone, Serialize)]
pub struct ProposalMessage<TAddr> {
    pub block: Block<TAddr>,
}
