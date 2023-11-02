//   Copyright 2023 OnSight Tech Services LLC
//   SPDX-License-Identifier: BSD-3-Clause

use serde::Serialize;
use taiji_dan_common_types::{Epoch, NodeHeight};
use taiji_dan_storage::consensus_models::QuorumCertificate;

use super::VoteMessage;

#[derive(Debug, Clone, Serialize)]
pub struct NewViewMessage<TAddr> {
    pub high_qc: QuorumCertificate<TAddr>,
    pub epoch: Epoch,
    pub new_height: NodeHeight,
    pub last_vote: Option<VoteMessage<TAddr>>,
}
